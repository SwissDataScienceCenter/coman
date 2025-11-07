use color_eyre::{Result, eyre::eyre};
use std::path::PathBuf;

use crate::{
    config::Config,
    cscs::{
        api_client::{CscsApi, FileSystemType, Job, JobDetail, System},
        oauth2::{
            CLIENT_ID_SECRET_NAME, CLIENT_SECRET_SECRET_NAME, client_credentials_login,
            finish_cscs_device_login, start_cscs_device_login,
        },
    },
    util::{
        keyring::{Secret, get_secret},
        types::DockerImageUrl,
    },
};
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

pub async fn cscs_login() -> Result<(Secret, Option<Secret>)> {
    let (details, verify_url) = start_cscs_device_login().await?;

    println!(
        "Please visit {} and authorize this application.",
        verify_url
    );
    open::that(verify_url.clone())
        .or_else(|_| {
            println!("Couldn't open browser, please navigate to {}", verify_url);
            std::io::Result::Ok(())
        })
        .unwrap();
    finish_cscs_device_login(details).await
}

pub async fn cscs_system_list() -> Result<Vec<System>> {
    match get_access_token().await {
        Ok(access_token) => {
            let api_client = CscsApi::new(access_token.0).unwrap();
            api_client.list_systems().await
        }
        Err(e) => Err(e),
    }
}

pub async fn cscs_job_list() -> Result<Vec<Job>> {
    match get_access_token().await {
        Ok(access_token) => {
            let api_client = CscsApi::new(access_token.0).unwrap();
            let config = Config::new().unwrap();
            api_client
                .list_jobs(&config.cscs.current_system, Some(true))
                .await
        }
        Err(e) => Err(e),
    }
}

pub async fn cscs_job_details(job_id: i64) -> Result<Option<JobDetail>> {
    match get_access_token().await {
        Ok(access_token) => {
            let api_client = CscsApi::new(access_token.0).unwrap();
            let config = Config::new().unwrap();
            api_client
                .get_job(&config.cscs.current_system, job_id)
                .await
        }
        Err(e) => Err(e),
    }
}

pub async fn cscs_start_job(
    script_file: Option<PathBuf>,
    image: Option<DockerImageUrl>,
    command: Option<Vec<String>>,
) -> Result<()> {
    match get_access_token().await {
        Ok(access_token) => {
            let api_client = CscsApi::new(access_token.0).unwrap();
            let config = Config::new().unwrap();
            let user_info = api_client.get_userinfo(&config.cscs.current_system).await?;
            let current_system = api_client.get_system(&config.cscs.current_system).await?;
            let scratch = match current_system {
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
                    return Err(eyre!(
                        "couldn't get system description for {}",
                        config.cscs.current_system
                    ));
                }
            };
            let base_path = scratch
                .join(user_info.name.clone())
                .join(config.cscs.name.clone().unwrap_or("coman".to_owned()));
            let mut tera = tera::Tera::default();

            let environment_path = base_path.join("environment.toml");
            let environment_template = config.cscs.edf_file_template;
            tera.add_raw_template("environment.toml", &environment_template)?;

            let docker_image = image.unwrap_or(config.cscs.image.try_into()?);
            let meta = docker_image.inspect().await?;
            if let Some(system_info) = config.cscs.systems.get(&config.cscs.current_system) {
                let mut compatible = false;
                for sys_platform in system_info.architecture.iter() {
                    if meta.platforms.contains(&sys_platform.clone().into()) {
                        compatible = true;
                    }
                }

                if !compatible {
                    return Err(eyre!(
                        "System {} only supports images with architecture(s) '{}' but the supplied image is for architecture(s) '{}'",
                        config.cscs.current_system,
                        system_info.architecture.join(","),
                        meta.platforms
                            .iter()
                            .map(|p| p.to_string())
                            .collect::<Vec<String>>()
                            .join(",")
                    ));
                }
            }

            let mut context = tera::Context::new();
            context.insert("edf_image", &docker_image.to_edf());
            let environment_file = tera.render("environment.toml", &context)?;
            api_client
                .mkdir(&config.cscs.current_system, base_path.clone())
                .await?;
            api_client
                .chmod(&config.cscs.current_system, base_path.clone(), "700")
                .await?;
            api_client
                .upload(
                    &config.cscs.current_system,
                    environment_path.clone(),
                    environment_file.into_bytes(),
                )
                .await?;

            // upload script
            let script_path = base_path.join("script.sh");
            let script_template = script_file
                .map(std::fs::read_to_string)
                .unwrap_or(Ok(config.cscs.sbatch_script_template))?;
            tera.add_raw_template("script.sh", &script_template)?;
            let name = config
                .cscs
                .name
                .unwrap_or(format!("{}-coman", user_info.name));
            let mut context = tera::Context::new();
            context.insert("name", &name);
            context.insert("command", &command.unwrap_or(config.cscs.command).join(" "));
            context.insert("environment_file", &environment_path);
            let script = tera.render("script.sh", &context)?;
            api_client
                .upload(
                    &config.cscs.current_system,
                    script_path.clone(),
                    script.into_bytes(),
                )
                .await?;

            // start job
            api_client
                .start_job(&config.cscs.current_system, &name, script_path)
                .await?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}
