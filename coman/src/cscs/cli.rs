use color_eyre::{Result, eyre::Context};
use inquire::{Password, Text};
use std::path::PathBuf;

use crate::{
    cscs::{
        handlers::{
            cscs_job_stop, cscs_job_details, cscs_job_list, cscs_start_job, cscs_system_list,
        },
        oauth2::{CLIENT_ID_SECRET_NAME, CLIENT_SECRET_SECRET_NAME, client_credentials_login},
    },
    util::{
        keyring::{Secret, get_secret, store_secret},
        types::DockerImageUrl,
    },
};

pub(crate) async fn cli_cscs_login() -> Result<()> {
    let client_id = match get_secret(CLIENT_ID_SECRET_NAME).await? {
        Some(client_id) => client_id,
        None => {
            let client_id = Text::new("Client Id:").prompt()?;
            let client_id_secret = Secret::new(client_id);
            store_secret(CLIENT_ID_SECRET_NAME, client_id_secret.clone()).await?;
            client_id_secret
        }
    };
    let client_secret = match get_secret(CLIENT_SECRET_SECRET_NAME).await? {
        Some(client_secret) => client_secret,
        None => {
            let client_secret = Password::new("Client Secret:").prompt()?;
            let client_secret_secret = Secret::new(client_secret);
            store_secret(CLIENT_SECRET_SECRET_NAME, client_secret_secret.clone()).await?;
            client_secret_secret
        }
    };

    match client_credentials_login(client_id, client_secret).await {
        Ok(_) => {
            println!("Successfully logged in");
        }
        Err(e) => Err(e).wrap_err("couldn't get acccess token")?,
    };
    Ok(())
}
pub(crate) async fn cli_cscs_job_list() -> Result<()> {
    match cscs_job_list().await {
        Ok(jobs) => {
            let mut table = tabled::Table::new(jobs);
            table.with(tabled::settings::Style::modern());
            println!("{}", table);
            Ok(())
        }
        Err(e) => Err(e),
    }
}
pub(crate) async fn cli_cscs_job_detail(job_id: i64) -> Result<()> {
    match cscs_job_details(job_id).await {
        Ok(Some(job)) => {
            let data = &[
                ("Id", job.id.to_string()),
                ("Name", job.name),
                (
                    "Start Date",
                    job.start_date
                        .map(|dt| dt.to_string())
                        .unwrap_or("".to_owned()),
                ),
                (
                    "End Date",
                    job.end_date
                        .map(|dt| dt.to_string())
                        .unwrap_or("".to_owned()),
                ),
                ("Status", job.status.to_string()),
                ("Status Reason", job.status_reason),
                ("Exit Code", job.exit_code.to_string()),
                ("stdin", job.stdin),
                ("stdout", job.stdout),
                ("stderr", job.stderr),
            ];
            let mut table = tabled::Table::nohead(data);
            table.with(tabled::settings::Style::modern());
            println!("{}", table);
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(e) => Err(e),
    }
}

pub(crate) async fn cli_cscs_job_start(
    script_file: Option<PathBuf>,
    image: Option<DockerImageUrl>,
    command: Option<Vec<String>>,
) -> Result<()> {
    cscs_start_job(script_file, image, command).await
}

pub(crate) async fn cli_cscs_job_stop(job_id: i64) -> Result<()> {
    cscs_job_stop(job_id).await
}

pub(crate) async fn cli_cscs_system_list() -> Result<()> {
    match cscs_system_list().await {
        Ok(systems) => {
            let mut table = tabled::Table::new(systems);
            table.with(tabled::settings::Style::modern());
            println!("{}", table);
            Ok(())
        }
        Err(e) => Err(e),
    }
}
