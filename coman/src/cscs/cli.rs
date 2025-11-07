use color_eyre::{
    Result,
    eyre::Context,
};
use std::path::PathBuf;

use crate::{
    cscs::{
        handlers::{cscs_job_details, cscs_job_list, cscs_login, cscs_start_job, cscs_system_list},
        oauth2::{
            ACCESS_TOKEN_SECRET_NAME, REFRESH_TOKEN_SECRET_NAME,
        },
    },
    util::{
        keyring::store_secret,
        types::DockerImageUrl,
    },
};

pub(crate) async fn cli_cscs_login() -> Result<()> {
    match cscs_login().await {
        Ok(result) => {
            store_secret(ACCESS_TOKEN_SECRET_NAME, result.0).await?;
            if let Some(refresh_token) = result.1 {
                store_secret(REFRESH_TOKEN_SECRET_NAME, refresh_token).await?;
            }
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
