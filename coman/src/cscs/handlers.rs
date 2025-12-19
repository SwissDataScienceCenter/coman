#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt;
#[cfg(target_family = "windows")]
use std::os::windows::fs::MetadataExt;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use color_eyre::{Result, eyre::eyre};
use eyre::Context;
use reqwest::Url;

use super::api_client::client::{EdfSpec, ScriptSpec};
use crate::{
    config::{ComputePlatform, Config},
    cscs::{
        api_client::{
            client::{CscsApi, JobStartOptions},
            types::{FileStat, FileSystemType, Job, JobDetail, PathEntry, PathType, S3Upload, System, UserInfo},
        },
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
                .list_jobs(&system.unwrap_or(config.values.cscs.current_system), Some(true))
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
) -> Result<Option<PathBuf>> {
    if options.no_ssh {
        return Ok(None);
    }

    let ssh_key = if let Some(path) = options.ssh_key.clone() {
        path.canonicalize().map(Some).wrap_err("couldn't get ssh key path")?
    } else {
        // try to figure our ssh key
        let ssh_dir = dirs::home_dir().ok_or(eyre!("couldn't find home dir"))?.join(".ssh");
        let mut ssh_path = None;
        for file in ["id_dsa.pub", "id_ecdsa.pub", "id_rsa.pub"] {
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
                .await?;
            Ok(Some(remote_path))
        }
        None => Err(eyre!("couldn't find ssh public key, use `--ssh_key` to specify it")),
    }
}

async fn handle_edf(
    api_client: &CscsApi,
    base_path: &Path,
    current_system: &str,
    envvars: &HashMap<String, String>,
    workdir: &str,
    options: &JobStartOptions,
) -> Result<PathBuf> {
    let config = Config::new().unwrap();
    let environment_path = base_path.join("environment.toml");
    match options.edf_spec.clone() {
        EdfSpec::Generate => {
            let mut tera = tera::Tera::default();

            let environment_template = &config.values.cscs.edf_file_template;
            tera.add_raw_template("environment.toml", environment_template)?;
            let mut mount: HashMap<String, String> = options.mount.clone().into_iter().collect();
            mount.entry("${SCRATCH}".to_owned()).or_insert("/scratch".to_owned());

            let docker_image = options
                .image
                .clone()
                .unwrap_or(config.values.cscs.image.clone().try_into()?);
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

            let ssh_path = setup_ssh(api_client, base_path, current_system, options).await?;

            let mut context = tera::Context::new();
            context.insert("edf_image", &docker_image.to_edf());
            context.insert("container_workdir", &workdir);
            context.insert("env", &envvars);
            context.insert("mount", &mount);
            context.insert("ssh_public_key", &ssh_path);

            let environment_file = tera.render("environment.toml", &context)?;
            api_client.mkdir(current_system, base_path.to_path_buf()).await?;
            api_client.chmod(current_system, base_path.to_path_buf(), "700").await?;
            api_client
                .upload(current_system, environment_path.clone(), environment_file.into_bytes())
                .await?;
            Ok(environment_path)
        }
        EdfSpec::Local(local_path) => {
            let environment_file = std::fs::read_to_string(local_path.clone())?;
            api_client.mkdir(current_system, base_path.to_path_buf()).await?;
            api_client.chmod(current_system, base_path.to_path_buf(), "700").await?;
            api_client
                .upload(current_system, environment_path.clone(), environment_file.into_bytes())
                .await?;
            Ok(environment_path)
        }
        EdfSpec::Remote(path) => Ok(path),
    }
}
async fn handle_script(
    api_client: &CscsApi,
    job_name: &str,
    base_path: &Path,
    current_system: &str,
    environment_path: &Path,
    workdir: &str,
    options: &JobStartOptions,
) -> Result<PathBuf> {
    let config = Config::new().unwrap();
    let script_path = base_path.join("script.sh");
    match options.script_spec.clone() {
        ScriptSpec::Generate => {
            let script_template = config.values.cscs.sbatch_script_template;
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
            let script = tera.render("script.sh", &context)?;
            api_client
                .upload(current_system, script_path.clone(), script.into_bytes())
                .await?;

            Ok(script_path)
        }
        ScriptSpec::Local(local_path) => {
            let script = std::fs::read_to_string(local_path)?;
            api_client
                .upload(current_system, script_path.clone(), script.into_bytes())
                .await?;

            Ok(script_path)
        }
        ScriptSpec::Remote(script_path) => Ok(script_path),
    }
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
            let config = Config::new().unwrap();
            let current_system = &system.unwrap_or(config.values.cscs.current_system);
            let account = account.or(config.values.cscs.account);
            let user_info = api_client.get_userinfo(current_system).await?;
            let job_name = name
                .or(config.values.name)
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
                .unwrap_or(config.values.cscs.workdir.unwrap_or("/scratch".to_owned()));
            let base_path = scratch.join(user_info.name.clone()).join(&job_name);

            let mut envvars = config.values.cscs.env.clone();
            envvars.extend(options.env.clone());

            let environment_path = handle_edf(
                &api_client,
                &base_path,
                current_system,
                &envvars,
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
                &container_workdir,
                &options,
            )
            .await?;

            // start job
            api_client
                .start_job(current_system, account, &job_name, script_path, envvars, options)
                .await?;
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
