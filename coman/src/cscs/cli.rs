use std::path::PathBuf;

use color_eyre::{Result, eyre::Context};
use inquire::{Password, Text};

use crate::{
    cscs::handlers::{
        cscs_job_cancel, cscs_job_details, cscs_job_list, cscs_job_log, cscs_login, cscs_start_job, cscs_system_list,
        cscs_system_set,
    },
    util::types::DockerImageUrl,
};

pub(crate) async fn cli_cscs_login() -> Result<()> {
    let client_id = Text::new("Client Id:").prompt()?;
    let client_secret = Password::new("Client Secret:").prompt()?;

    match cscs_login(client_id, client_secret).await {
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
                    job.start_date.map(|dt| dt.to_string()).unwrap_or("".to_owned()),
                ),
                (
                    "End Date",
                    job.end_date.map(|dt| dt.to_string()).unwrap_or("".to_owned()),
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

pub(crate) async fn cli_cscs_job_log(job_id: i64) -> Result<()> {
    match cscs_job_log(job_id).await {
        Ok(content) => {
            println!("{}", content);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub(crate) async fn cli_cscs_job_start(
    script_file: Option<PathBuf>,
    image: Option<DockerImageUrl>,
    command: Option<Vec<String>>,
    workdir: Option<String>,
    env: Vec<(String, String)>,
) -> Result<()> {
    cscs_start_job(script_file, image, command, workdir, env).await
}

pub(crate) async fn cli_cscs_job_cancel(job_id: i64) -> Result<()> {
    cscs_job_cancel(job_id).await
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
pub(crate) async fn cli_cscs_set_system(system_name: String, global: bool) -> Result<()> {
    cscs_system_set(system_name, global).await
}
