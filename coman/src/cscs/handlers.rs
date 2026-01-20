#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt;
#[cfg(target_family = "windows")]
use std::os::windows::fs::MetadataExt;
use std::{
    collections::{HashMap, HashSet},
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

use base64::prelude::*;
use color_eyre::{Result, eyre::eyre};
use eyre::Context;
use futures::StreamExt;
use iroh::{Endpoint, EndpointId, SecretKey, protocol::Router};
use itertools::Itertools;
use regex::Regex;
use reqwest::Url;
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};

use super::api_client::client::{EdfSpec, ScriptSpec};
use crate::{
    config::{ComputePlatform, Config, get_data_dir},
    cscs::{
        api_client::{
            client::{CscsApi, JobStartOptions},
            types::{
                FileStat, FileSystemType, Job, JobDetail, JobStatus, PathEntry, PathType, S3Upload, System, UserInfo,
            },
        },
        cli::upload_chunk,
        oauth2::{
            CLIENT_ID_SECRET_NAME, CLIENT_SECRET_SECRET_NAME, client_credentials_login, finish_cscs_device_login,
            start_cscs_device_login,
        },
    },
    util::keyring::{Secret, get_secret, store_secret},
};

const CSCS_MAX_DIRECT_SIZE: usize = 5242880;

async fn get_access_token() -> Result<Secret> {
    let client_id = match get_secret(CLIENT_ID_SECRET_NAME).await {
        Ok(Some(client_id)) => client_id,
        Ok(None) => Err(eyre!("not logged in"))?,
        Err(e) => Err(e)?,
    };
    let client_secret = match get_secret(CLIENT_SECRET_SECRET_NAME).await {
        Ok(Some(client_secret)) => client_secret,
        Ok(None) => Err(eyre!("not logged in"))?,
        Err(e) => Err(e)?,
    };
    let token = client_credentials_login(client_id, client_secret).await?;
    Ok(token.0)
}
pub(crate) async fn cscs_login(client_id: String, client_secret: String) -> Result<()> {
    let client_id_secret = Secret::new(client_id);
    store_secret(CLIENT_ID_SECRET_NAME, client_id_secret.clone()).await?;
    let client_secret_secret = Secret::new(client_secret);
    store_secret(CLIENT_SECRET_SECRET_NAME, client_secret_secret.clone()).await?;
    client_credentials_login(client_id_secret, client_secret_secret)
        .await
        .map(|_| ())
}

#[allow(dead_code)]
pub async fn cscs_login_device_code() -> Result<(Secret, Option<Secret>)> {
    let (details, verify_url) = start_cscs_device_login().await?;

    println!("Please visit {} and authorize this application.", verify_url);
    open::that(verify_url.clone())
        .or_else(|_| {
            println!("Couldn't open browser, please navigate to {}", verify_url);
            std::io::Result::Ok(())
        })
        .unwrap();
    finish_cscs_device_login(details).await
}

pub async fn cscs_system_list(platform: Option<ComputePlatform>) -> Result<Vec<System>> {
    match get_access_token().await {
        Ok(access_token) => {
            let api_client = CscsApi::new(access_token.0, platform).unwrap();
            api_client.list_systems().await
        }
        Err(e) => Err(e),
    }
}

pub async fn cscs_system_set(system_name: String, global: bool) -> Result<()> {
    let mut config = Config::new()?;
    config.set("cscs.current_system", system_name, global)
}

pub async fn cscs_job_list(system: Option<String>, platform: Option<ComputePlatform>) -> Result<Vec<Job>> {
    match get_access_token().await {
        Ok(access_token) => {
            let api_client = CscsApi::new(access_token.0, platform).unwrap();
            let config = Config::new().unwrap();
            api_client
                .list_jobs(&system.unwrap_or(config.values.cscs.current_system), None)
                .await
        }
        Err(e) => Err(e),
    }
}

pub async fn cscs_job_details(
    job_id: i64,
    system: Option<String>,
    platform: Option<ComputePlatform>,
) -> Result<Option<JobDetail>> {
    match get_access_token().await {
        Ok(access_token) => {
            let api_client = CscsApi::new(access_token.0, platform).unwrap();
            let config = Config::new().unwrap();
            api_client
                .get_job(&system.unwrap_or(config.values.cscs.current_system), job_id)
                .await
        }
        Err(e) => Err(e),
    }
}

pub async fn cscs_job_log(
    job_id: i64,
    stderr: bool,
    system: Option<String>,
    platform: Option<ComputePlatform>,
) -> Result<String> {
    match get_access_token().await {
        Ok(access_token) => {
            let api_client = CscsApi::new(access_token.0, platform).unwrap();
            let config = Config::new().unwrap();
            let current_system = &system.unwrap_or(config.values.cscs.current_system);
            let job = api_client.get_job(current_system, job_id).await?;
            if job.is_none() {
                return Err(eyre!("couldn't find job {}", job_id));
            }
            let path = if stderr {
                job.unwrap().stderr
            } else {
                job.unwrap().stdout
            };
            if path.is_empty() {
                return Err(eyre!(
                    "No {} log exists for job {}",
                    if stderr { "stderr" } else { "stdout" },
                    job_id
                ));
            }

            let path = PathBuf::from(path);
            api_client.tail(current_system, path, 100).await
        }
        Err(e) => Err(e),
    }
}

pub async fn cscs_port_forward(
    job_id: i64,
    source_port: u16,
    destination_port: u16,
    system: Option<String>,
) -> Result<()> {
    let data_dir = get_data_dir();
    let config = Config::new().unwrap();
    let current_system = &system.unwrap_or(config.values.cscs.current_system);
    let job_info = cscs_job_details(job_id, Some(current_system.clone()), None).await?;
    if job_info.is_none() {
        return Err(eyre!("remote job does not exist!"));
    } else if let Some(job_info) = job_info
        && job_info.status != JobStatus::Running
    {
        return Err(eyre!("remote job is not in running state, connection not available"));
    }
    let endpoint_id = std::fs::read_to_string(data_dir.join(format!("{}_{}.endpoint", current_system, job_id)))?;
    let endpoint_id = EndpointId::from_str(if endpoint_id.len() == 64 {
        &endpoint_id
    } else if endpoint_id.len() > 64 {
        &endpoint_id[endpoint_id.len() - 64..]
    } else {
        return Err(eyre!("invalid endpoint id length"));
    })?;
    let listener = TcpListener::bind(format!("127.0.0.1:{source_port}")).await?;
    println!("forwarding connection for port {source_port}");

    loop {
        let (socket, _) = listener.accept().await?;
        process_port_forward(endpoint_id, destination_port, socket).await?;
    }
}

async fn process_port_forward(endpoint_id: EndpointId, destination_port: u16, mut socket: TcpStream) -> Result<()> {
    println!("accepted connection for destination port {destination_port}");
    let alpn: Vec<u8> = format!("/coman/{destination_port}").into_bytes();
    let secret_key = SecretKey::generate(&mut rand::rng());
    let endpoint = Endpoint::builder().secret_key(secret_key).bind().await?;
    Router::builder(endpoint.clone()).spawn(); // start local iroh listener

    match endpoint.connect(endpoint_id, &alpn).await {
        Ok(connection) => {
            let (mut iroh_send, mut iroh_recv) = connection.open_bi().await?;
            let (mut local_read, mut local_write) = socket.split();
            let a_to_b = async move { tokio::io::copy(&mut local_read, &mut iroh_send).await };
            let b_to_a = async move { tokio::io::copy(&mut iroh_recv, &mut local_write).await };
            println!("connection open");

            tokio::select! {
                result = a_to_b => {
                    let _ = result;
                },
                result = b_to_a => {
                    let _ = result;
                },
            };
            println!("connection closed");

            Ok(())
        }
        Err(e) => Err(e).wrap_err("couldn't establish tunnel to remote"),
    }
}

pub async fn cscs_job_cancel(job_id: i64, system: Option<String>, platform: Option<ComputePlatform>) -> Result<()> {
    match get_access_token().await {
        Ok(access_token) => {
            let api_client = CscsApi::new(access_token.0, platform).unwrap();
            let config = Config::new().unwrap();
            api_client
                .cancel_job(&system.unwrap_or(config.values.cscs.current_system), job_id)
                .await
        }
        Err(e) => Err(e),
    }
}

async fn setup_ssh(
    api_client: &CscsApi,
    base_path: &Path,
    current_system: &str,
    options: &JobStartOptions,
    config: &Config,
) -> Result<Option<(PathBuf, SecretKey)>> {
    if options.no_ssh {
        return Ok(None);
    }
    let secret = SecretKey::generate(&mut rand::rng());

    let ssh_key = if let Some(path) = options.ssh_key.clone().or(config.values.cscs.ssh_key.clone()) {
        path.canonicalize().map(Some).wrap_err("couldn't get ssh key path")?
    } else {
        // try to figure our ssh key
        let ssh_dir = dirs::home_dir().ok_or(eyre!("couldn't find home dir"))?.join(".ssh");
        let mut ssh_path = None;
        for file in ["id_dsa.pub", "id_ecdsa.pub", "id_rsa.pub", "id_ed25519.pub"] {
            let path = ssh_dir.join(file);
            if path.exists() {
                ssh_path = Some(path);
                break;
            }
        }
        ssh_path
    };

    match ssh_key {
        Some(path) => {
            let filename = path.file_name().ok_or(eyre!("couldn't get filename of ssh key"))?;
            let remote_path = base_path.join(filename);
            let public_key = std::fs::read_to_string(path.clone())?;

            api_client
                .upload(current_system, remote_path.clone(), public_key.into_bytes())
                .await
                .wrap_err(eyre!("couldn't upload ssh public key"))?;
            Ok(Some((remote_path, secret)))
        }
        None => Err(eyre!("couldn't find ssh public key, use `--ssh_key` to specify it")),
    }
}

async fn garbage_collect_ssh(api_client: &CscsApi, current_system: &str) -> Result<()> {
    let data_dir = get_data_dir();
    if !data_dir.exists() {
        return Ok(());
    }
    let jobs = api_client.list_jobs(current_system, None).await?;
    let job_entries: HashSet<_> = jobs
        .iter()
        .filter(|j| j.status == JobStatus::Pending || j.status == JobStatus::Running)
        .map(|j| format!("{}_{}", current_system, j.id))
        .collect();
    let outdated_endpoints: Vec<_> = std::fs::read_dir(&data_dir)?
        .filter(|d| {
            d.as_ref().is_ok_and(|e| {
                e.path().is_file()
                    && e.file_name().to_string_lossy().ends_with(".endpoint")
                    && e.file_name().to_string_lossy().starts_with(current_system)
                    && !job_entries.contains(e.file_name().to_string_lossy().split_once('.').unwrap().0)
            })
        })
        .map(|d| d.unwrap())
        .collect();

    // delete connection files
    for d in outdated_endpoints.iter() {
        std::fs::remove_file(d.path())?;
    }

    // cleanup ssh config
    let coman_ssh_config_path = data_dir.join("ssh_config");
    if !coman_ssh_config_path.exists() {
        return Ok(());
    }
    let mut ssh_content = std::fs::read_to_string(&coman_ssh_config_path)?;
    for d in outdated_endpoints {
        let re = Regex::new(
            format!(
                r"(?ms)#Start {0}_[^\s]_{1}.*?#End {0}_[^\s]_{1}\n",
                current_system,
                d.file_name()
                    .to_string_lossy()
                    .split_once('.')
                    .unwrap()
                    .0
                    .rsplit('_')
                    .next()
                    .unwrap()
            )
            .as_str(),
        )?;
        ssh_content = re.replace(&ssh_content, "").to_string();
    }

    std::fs::write(coman_ssh_config_path, ssh_content)?;

    Ok(())
}

async fn store_ssh_information(
    current_system: &str,
    user_info: &UserInfo,
    job_id: &i64,
    job_name: &str,
    secret_key: &SecretKey,
) -> Result<String> {
    let data_dir = get_data_dir();
    std::fs::write(
        data_dir.join(format!("{}_{}.endpoint", current_system, job_id)),
        format!("{}", secret_key.public()),
    )?;
    let coman_ssh_config_path = data_dir.join("ssh_config");
    let coman_ssh_config = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(coman_ssh_config_path.clone())?;
    let connection_name = format!("{}-{}-{}", current_system, job_name, job_id);
    let mut writer = BufWriter::new(coman_ssh_config);
    write!(
        writer,
        "\n#Start {0}\nHost {0}\n    Hostname {1}\n    User {2}\n    ProxyCommand coman proxy {3} {4}\n#End {0}",
        connection_name,
        secret_key.public(),
        user_info.name,
        current_system,
        job_id
    )?;
    let ssh_dir = dirs::home_dir().ok_or(eyre!("couldn't find home dir"))?.join(".ssh");
    let ssh_config_path = ssh_dir.join("config");
    let mut ssh_config = std::fs::OpenOptions::new()
        .read(true)
        .append(true)
        .open(ssh_config_path)?;
    let mut content = String::new();
    ssh_config.read_to_string(&mut content)?;
    if !content.contains(&format!("Include {}", coman_ssh_config_path.clone().display())) {
        let mut writer = BufWriter::new(ssh_config);
        write!(
            writer,
            "\n\n#coman include\nMatch all\nInclude {}",
            coman_ssh_config_path.display()
        )?;
    }
    Ok(connection_name)
}

async fn inject_coman_squash(
    api_client: &CscsApi,
    base_path: &Path,
    current_system: &str,
    options: &JobStartOptions,
) -> Result<Option<PathBuf>> {
    if options.no_coman {
        return Ok(None);
    }
    let config = Config::new().unwrap();
    let local_squash_path = match config.values.coman_squash_path.clone() {
        Some(path) => path,
        None => {
            //download from github for architecture
            let system = config
                .values
                .cscs
                .systems
                .get(current_system)
                .ok_or(eyre!("couldn't find architecture for system {}", current_system))?;
            let architecture = system
                .architecture
                .first()
                .ok_or(eyre!("no architecture set for {}", current_system))?;
            let target_path = get_data_dir().join(format!("coman_{}.sqsh", architecture));
            if !target_path.exists() {
                let url = match architecture.as_str() {
                    "arm64" => {
                        "https://github.com/SwissDataScienceCenter/coman/releases/latest/download/coman_Linux-aarch64.sqsh"
                    }
                    "amd64" => {
                        "https://github.com/SwissDataScienceCenter/coman/releases/latest/download/coman_Linux-x86_64.sqsh"
                    }
                    _ => {
                        return Err(eyre!("unsupported architecture {}", architecture));
                    }
                };
                let mut out = File::create(target_path.clone()).await?;
                let resp = reqwest::get(url).await?;
                match resp.error_for_status() {
                    Ok(resp) => {
                        let mut stream = resp.bytes_stream();
                        while let Some(chunk_result) = stream.next().await {
                            let chunk = chunk_result?;
                            out.write_all(&chunk).await?;
                        }
                        out.flush().await?;
                    }
                    Err(e) => return Err(eyre!("couldn't download coman squash file: {}", e)),
                }
            }
            target_path
        }
    };
    let target = base_path.join("coman.sqsh");
    let file_meta = std::fs::metadata(local_squash_path.clone())?;

    #[cfg(target_family = "unix")]
    let size = file_meta.size() as usize;

    #[cfg(target_family = "windows")]
    let size = file_meta.file_size() as usize;

    let response = api_client.list_path(current_system, target.clone()).await;
    if let Ok(existing) = response
        && !existing.is_empty()
    {
        //squash file already present on remote, don't upload if it's the same
        let entry = existing.first().unwrap();
        if entry.size.unwrap_or_default() == size {
            return Ok(Some(target));
        } else {
            // remove file before upload
            api_client.rm_path(current_system, target.clone()).await?;
        }
    }
    //upload squash file
    let transfer_data = api_client
        .transfer_upload(current_system, config.values.cscs.account, target.clone(), size as i64)
        .await
        .wrap_err(eyre!("couldn't upload coman squash file"))?;
    let mut etags: Vec<String> = Vec::new();
    let client = reqwest::Client::new();
    let num_parts = transfer_data.1.num_parts;
    for (chunk_id, transfer_url) in transfer_data.1.parts_upload_urls.into_iter().enumerate() {
        println!(
            "Uploading part {}/{} ({}Mb)",
            chunk_id + 1,
            num_parts,
            transfer_data.1.part_size / 1024 / 1024
        );
        let etag = upload_chunk(
            local_squash_path.clone(),
            (chunk_id as u64) * transfer_data.1.part_size,
            transfer_data.1.part_size,
            transfer_url,
        )
        .await?;
        etags.push(etag);
    }

    let body = etags
        .into_iter()
        .enumerate()
        .map(|(i, etag)| (i + 1, etag))
        .map(|(i, etag)| format!("<Part><PartNumber>{}</PartNumber><ETag>{}</ETag></Part>", i, etag))
        .join("");
    let body = format!(
        "<CompleteMultipartUpload xmlns=\"http://s3.amazonaws.com/doc/2006-03-01\">{}</CompleteMultipartUpload>",
        body
    );
    let req = client.post(transfer_data.1.complete_upload_url).body(body).build()?;
    let resp = client.execute(req).await?;
    resp.error_for_status()?;
    // wait for transfer job to finish
    loop {
        match cscs_job_details(transfer_data.0, Some(current_system.to_string()), None).await? {
            Some(JobDetail {
                status: JobStatus::Finished,
                ..
            }) => break,
            Some(JobDetail {
                status: JobStatus::Cancelled | JobStatus::Failed | JobStatus::Timeout,
                ..
            }) => {
                return Err(eyre!(
                    "Uploading coman sqsh failed, check job {} for more details",
                    transfer_data.0
                ));
            }
            Some(_) | None => {}
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Ok(Some(target))
}

#[allow(clippy::too_many_arguments)]
async fn handle_edf(
    api_client: &CscsApi,
    base_path: &Path,
    current_system: &str,
    envvars: &HashMap<String, String>,
    coman_squash: &Option<PathBuf>,
    ssh_public_key_path: &Option<PathBuf>,
    iroh_secret: &Option<SecretKey>,
    workdir: &str,
    options: &JobStartOptions,
) -> Result<PathBuf> {
    let config = Config::new().unwrap();
    let environment_path = base_path.join("environment.toml");

    let environment_template = match options.edf_spec.clone() {
        EdfSpec::Generate => config.values.cscs.edf_file_template,
        EdfSpec::Local(local_path) => std::fs::read_to_string(local_path.clone())?,
        EdfSpec::Remote(path) => return Ok(path),
    };

    let mut tera = tera::Tera::default();

    tera.add_raw_template("environment.toml", &environment_template)?;
    let mut mount: HashMap<String, String> = options.mount.clone().into_iter().collect();
    mount.entry("${SCRATCH}".to_owned()).or_insert("/scratch".to_owned());

    let mut context = tera::Context::new();

    // check and validate image if set
    let docker_image = if let Some(image) = options.image.clone() {
        Some(image)
    } else if let Some(image) = config.values.cscs.image {
        let image = image.try_into()?;
        Some(image)
    } else {
        None
    };
    if let Some(docker_image) = docker_image {
        let meta = docker_image.inspect().await?;
        if let Some(system_info) = config.values.cscs.systems.get(current_system) {
            let mut compatible = false;
            for sys_platform in system_info.architecture.iter() {
                if meta.platforms.contains(&sys_platform.clone().into()) {
                    compatible = true;
                }
            }

            if !compatible {
                return Err(eyre!(
                    "System {} only supports images with architecture(s) '{}' but the supplied image is for architecture(s) '{}'",
                    current_system,
                    system_info.architecture.join(","),
                    meta.platforms
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<String>>()
                        .join(",")
                ));
            }
        }

        context.insert("edf_image", &docker_image.to_edf());
    }

    if !options.port_forward.is_empty() {
        let port_forward = options.port_forward.iter().map(|f| f.to_string()).join(",");
        context.insert("port_forward", &port_forward);
    } else if !config.values.cscs.port_forward.is_empty() {
        let port_forward = config.values.cscs.port_forward.iter().map(|f| f.to_string()).join(",");
        context.insert("port_forward", &port_forward);
    }

    context.insert("container_workdir", &workdir);
    context.insert("env", &envvars);
    context.insert("mount", &mount);
    context.insert("ssh_public_key", &ssh_public_key_path);
    context.insert("coman_squash", &coman_squash);
    if let Some(iroh_secret) = iroh_secret {
        // set iroh secret key
        let encoded_secret = BASE64_STANDARD.encode(iroh_secret.to_bytes());
        context.insert("iroh_secret", &encoded_secret);
    }

    let environment_file = tera.render("environment.toml", &context)?;
    api_client.mkdir(current_system, base_path.to_path_buf()).await?;
    api_client.chmod(current_system, base_path.to_path_buf(), "700").await?;
    api_client
        .upload(current_system, environment_path.clone(), environment_file.into_bytes())
        .await?;
    Ok(environment_path)
}

#[allow(clippy::too_many_arguments)]
async fn handle_script(
    api_client: &CscsApi,
    job_name: &str,
    base_path: &Path,
    current_system: &str,
    environment_path: &Path,
    coman_squash: Option<PathBuf>,
    workdir: &str,
    options: &JobStartOptions,
) -> Result<PathBuf> {
    let config = Config::new().unwrap();
    let script_path = base_path.join("script.sh");
    let script_template = match options.script_spec.clone() {
        ScriptSpec::Generate => config.values.cscs.sbatch_script_template,
        ScriptSpec::Local(local_path) => std::fs::read_to_string(local_path)?,
        ScriptSpec::Remote(script_path) => return Ok(script_path),
    };

    let mut tera = tera::Tera::default();
    tera.add_raw_template("script.sh", &script_template)?;
    let mut context = tera::Context::new();
    context.insert("name", &job_name);
    context.insert(
        "command",
        &options.command.clone().unwrap_or(config.values.cscs.command).join(" "),
    );
    context.insert("environment_file", &environment_path.to_path_buf());
    context.insert("container_workdir", &workdir);
    if let Some(path) = coman_squash {
        context.insert("coman_squash", &path);
    }
    let script = tera.render("script.sh", &context)?;
    api_client
        .upload(current_system, script_path.clone(), script.into_bytes())
        .await?;

    Ok(script_path)
}

pub async fn cscs_job_start(
    name: Option<String>,
    options: JobStartOptions,
    system: Option<String>,
    platform: Option<ComputePlatform>,
    account: Option<String>,
) -> Result<()> {
    match get_access_token().await {
        Ok(access_token) => {
            let api_client = CscsApi::new(access_token.0, platform).unwrap();
            let config = Config::new()?;
            let current_system = &system.unwrap_or(config.values.cscs.current_system.clone());
            let account = account.or(config.values.cscs.account.clone());
            let user_info = api_client.get_userinfo(current_system).await?;
            let job_name = name
                .or(config.values.name.clone())
                .unwrap_or(format!("{}-coman", user_info.name));
            let current_system_info = api_client.get_system(current_system).await?;
            let scratch = match current_system_info {
                Some(system) => PathBuf::from(
                    system
                        .file_systems
                        .iter()
                        .find(|fs| fs.data_type == FileSystemType::Scratch)
                        .ok_or(eyre!("couldn't find scratch space for system"))?
                        .path
                        .clone(),
                ),
                None => {
                    return Err(eyre!("couldn't get system description for {}", current_system));
                }
            };
            let container_workdir = options
                .container_workdir
                .clone()
                .unwrap_or(config.values.cscs.workdir.clone().unwrap_or("/scratch".to_owned()));
            let base_path = scratch.join(user_info.name.clone()).join(&job_name);
            api_client.mkdir(current_system, base_path.to_path_buf()).await?;
            api_client.chmod(current_system, base_path.to_path_buf(), "700").await?;

            let mut envvars = config.values.cscs.env.clone();
            envvars.extend(options.env.clone());

            let (ssh_public_key_path, secret_key) =
                setup_ssh(&api_client, &base_path, current_system, &options, &config)
                    .await?
                    .unzip();
            if ssh_public_key_path.is_none() {
                println!(
                    "Warning: No ssh key found, specify it with --ssh-key if you want to use ssh connections through coman"
                );
            }
            let coman_squash = inject_coman_squash(&api_client, &base_path, current_system, &options).await?;
            if coman_squash.is_none() {
                println!("Warning: coman squash wasn't templated and is needed for ssh through coman to work");
            }

            let environment_path = handle_edf(
                &api_client,
                &base_path,
                current_system,
                &envvars,
                &coman_squash,
                &ssh_public_key_path,
                &secret_key,
                &container_workdir,
                &options,
            )
            .await?;

            let script_path = handle_script(
                &api_client,
                &job_name,
                &base_path,
                current_system,
                &environment_path,
                coman_squash,
                &container_workdir,
                &options,
            )
            .await?;

            // start job
            let job_id = api_client
                .start_job(current_system, account, &job_name, script_path, envvars, options)
                .await?
                .ok_or(eyre!("didn't get job id for created job"))?;

            if let Some(secret_key) = secret_key {
                // store connection information in data dir and set up ssh connection
                garbage_collect_ssh(&api_client, current_system).await?;
                let connection_name =
                    store_ssh_information(current_system, &user_info, &job_id, &job_name, &secret_key).await?;
                println!("Use ssh {}@{} to connect to the job", user_info.name, connection_name);
            }

            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub async fn cscs_file_list(
    path: PathBuf,
    system: Option<String>,
    platform: Option<ComputePlatform>,
) -> Result<Vec<PathEntry>> {
    match get_access_token().await {
        Ok(access_token) => {
            let api_client = CscsApi::new(access_token.0, platform).unwrap();
            let config = Config::new().unwrap();
            api_client
                .list_path(&system.unwrap_or(config.values.cscs.current_system), path)
                .await
        }
        Err(e) => Err(e),
    }
}
pub async fn file_system_roots() -> Result<Vec<PathEntry>> {
    let config = Config::new().expect("couldn't load config");
    let user_info = cscs_user_info(None, None).await?;
    let systems = cscs_system_list(None).await?;
    let system = systems
        .iter()
        .find(|s| s.name == config.values.cscs.current_system)
        .unwrap_or_else(|| panic!("couldn't get info for system {}", config.values.cscs.current_system));
    let mut subpaths = vec![];
    for fs in system.file_systems.clone() {
        let entry = match cscs_stat_path(PathBuf::from(fs.path.clone()).join(user_info.name.clone()), None, None).await
        {
            Ok(Some(_)) => PathEntry {
                name: format!("{}/{}", fs.path.clone(), user_info.name),
                path_type: PathType::Directory,
                permissions: None,
                size: None,
            },
            _ => PathEntry {
                name: fs.path.clone(),
                path_type: PathType::Directory,
                permissions: None,
                size: None,
            },
        };
        subpaths.push(entry);
    }
    Ok(subpaths)
}

pub async fn cscs_file_delete(
    remote: PathBuf,
    system: Option<String>,
    platform: Option<ComputePlatform>,
) -> Result<()> {
    match get_access_token().await {
        Ok(access_token) => {
            let api_client = CscsApi::new(access_token.0, platform).unwrap();
            let config = Config::new().unwrap();
            let current_system = &system.unwrap_or(config.values.cscs.current_system);
            let paths = api_client.list_path(current_system, remote.clone()).await?;
            let path = paths.first().ok_or(eyre!("remote path doesn't exist"))?;
            if let PathType::Directory = path.path_type {
                return Err(eyre!("remote path must be a file, not directory"));
            }
            api_client.rm_path(current_system, remote).await?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub async fn cscs_file_download(
    remote: PathBuf,
    local: PathBuf,
    account: Option<String>,
    system: Option<String>,
    platform: Option<ComputePlatform>,
) -> Result<Option<(i64, Url, usize)>> {
    let local = if local.is_dir() {
        local.join(remote.file_name().ok_or(eyre!("couldn't get name of remote file"))?)
    } else {
        local
    };
    match get_access_token().await {
        Ok(access_token) => {
            let api_client = CscsApi::new(access_token.0, platform).unwrap();
            let config = Config::new().unwrap();
            let current_system = &system.unwrap_or(config.values.cscs.current_system);
            let paths = api_client.list_path(current_system, remote.clone()).await?;
            let path = paths.first().ok_or(eyre!("remote path doesn't exist"))?;
            if let PathType::Directory = path.path_type {
                return Err(eyre!("remote path must be a file, not directory"));
            }
            let size = path.size.ok_or(eyre!("couldn't determin download file size"))?;
            if size < CSCS_MAX_DIRECT_SIZE {
                // download directly
                let contents = api_client.download(current_system, remote).await?;
                std::fs::write(local, contents)?;
                Ok(None)
            } else {
                // download via s3
                let account = account.or(config.values.cscs.account);
                let job_data = api_client.transfer_download(current_system, account, remote).await?;
                Ok(Some((job_data.0, job_data.1, size)))
            }
        }
        Err(e) => Err(e),
    }
}
pub async fn cscs_file_upload(
    local: PathBuf,
    remote: PathBuf,
    account: Option<String>,
    system: Option<String>,
    platform: Option<ComputePlatform>,
) -> Result<Option<(i64, S3Upload)>> {
    match get_access_token().await {
        Ok(access_token) => {
            let api_client = CscsApi::new(access_token.0, platform).unwrap();
            let config = Config::new().unwrap();
            let current_system = &system.unwrap_or(config.values.cscs.current_system);
            let existing = api_client.list_path(current_system, remote.clone()).await?;
            let remote = if !existing.is_empty() {
                if existing.len() == 1 && existing[0].path_type == PathType::File {
                    return Err(eyre!("remote file already exists"));
                }
                remote.join(local.file_name().ok_or(eyre!("couldn't get filename for local file"))?)
            } else {
                remote
            };

            let file_meta = std::fs::metadata(local.clone())?;

            #[cfg(target_family = "unix")]
            let size = file_meta.size() as usize;

            #[cfg(target_family = "windows")]
            let size = file_meta.file_size() as usize;

            if size < CSCS_MAX_DIRECT_SIZE {
                // upload directly
                let contents = std::fs::read(local)?;
                api_client.upload(current_system, remote, contents).await?;
                Ok(None)
            } else {
                // upload via s3
                let account = account.or(config.values.cscs.account);
                let transfer_data = api_client
                    .transfer_upload(current_system, account, remote, size as i64)
                    .await?;

                Ok(Some(transfer_data))
            }
        }
        Err(e) => Err(e),
    }
}

pub async fn cscs_stat_path(
    path: PathBuf,
    system: Option<String>,
    platform: Option<ComputePlatform>,
) -> Result<Option<FileStat>> {
    match get_access_token().await {
        Ok(access_token) => {
            let api_client = CscsApi::new(access_token.0, platform).unwrap();
            let config = Config::new().unwrap();
            api_client
                .stat_path(&system.unwrap_or(config.values.cscs.current_system), path)
                .await
        }
        Err(e) => Err(e),
    }
}

pub async fn cscs_user_info(system: Option<String>, platform: Option<ComputePlatform>) -> Result<UserInfo> {
    match get_access_token().await {
        Ok(access_token) => {
            let api_client = CscsApi::new(access_token.0, platform).unwrap();
            let config = Config::new().unwrap();
            api_client
                .get_userinfo(&system.unwrap_or(config.values.cscs.current_system))
                .await
        }
        Err(e) => Err(e),
    }
}
