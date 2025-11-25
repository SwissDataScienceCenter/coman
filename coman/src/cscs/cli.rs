use std::{path::PathBuf, time::Duration};

use color_eyre::{Result, eyre::Context};
use eyre::eyre;
use inquire::{Password, Text};
use itertools::Itertools;
use reqwest::Url;

use crate::{
    cscs::{
        api_client::JobStatus,
        handlers::{
            cscs_file_download, cscs_file_list, cscs_file_upload, cscs_job_cancel, cscs_job_details, cscs_job_list,
            cscs_job_log, cscs_login, cscs_start_job, cscs_system_list, cscs_system_set,
        },
    },
    trace_dbg,
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

pub(crate) async fn cli_cscs_file_list(path: PathBuf) -> Result<()> {
    match cscs_file_list(path).await {
        Ok(path_entries) => {
            let mut table = tabled::Table::new(path_entries);
            table.with(tabled::settings::Style::empty());
            println!("{}", table);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub(crate) async fn cli_cscs_file_download(remote: PathBuf, local: PathBuf) -> Result<()> {
    match cscs_file_download(remote, local.clone()).await {
        Ok(None) => {
            println!("File successfully downloaded");
            Ok(())
        }
        Ok(Some(job_data)) => {
            // file is large, so we created a transfer job to s3 that we need to wait on
            // then we can download from s3
            println!("started s3 transfer job {}", job_data.0);
            let mut transfer_done = false;
            while !transfer_done {
                if let Some(job) = cscs_job_details(job_data.0).await? {
                    match job.status {
                        JobStatus::Pending | JobStatus::Running => {}
                        JobStatus::Finished => transfer_done = true,
                        JobStatus::Cancelled | JobStatus::Failed => return Err(eyre!("transfer job failed")),
                        JobStatus::Timeout => return Err(eyre!("transfer job timed out")),
                    }
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }

            // download from s3
            println!("Downloading file from s3, this might take a while");
            let credentials = s3::creds::Credentials::default()?;
            let url = Url::parse(&job_data.1)?;
            let region = s3::region::Region::Custom {
                region: "cscs-zonegroup".to_owned(),
                endpoint: "https://rgw.cscs.ch".to_owned(),
            };
            let mut segments = url.path_segments().unwrap();
            let bucket_name = segments.next().unwrap();
            let path = segments.join("/");
            let bucket = s3::bucket::Bucket::create_with_path_style(
                bucket_name,
                region,
                credentials,
                s3::BucketConfiguration::default(),
            )
            .await?
            .bucket;
            let mut async_output_file = tokio::fs::File::create(&local).await.expect("Unable to create file");
            let status = bucket.get_object_to_writer(path, &mut async_output_file).await?;
            println!("download finished with status {}", status);

            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub(crate) async fn cli_cscs_file_upload(local: PathBuf, remote: PathBuf) -> Result<()> {
    match cscs_file_upload(local, remote).await {
        Ok(None) => {
            println!("File successfully uploaded");
            Ok(())
        }
        Ok(Some(transfer_data)) => {
            println!("starting file transfer, this might take a while");
            let transfer_data = trace_dbg!(transfer_data);
            Ok(())
        }
        Err(e) => Err(e),
    }
}
