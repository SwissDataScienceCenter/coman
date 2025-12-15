use std::{
    io::{SeekFrom, Write},
    path::PathBuf,
    time::{Duration, Instant},
};

use color_eyre::{Result, eyre::Context};
use eyre::eyre;
use futures::StreamExt;
use inquire::{Password, Text};
use itertools::Itertools;
use reqwest::Url;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader},
};

use crate::{
    config::ComputePlatform,
    cscs::{
        api_client::{client::JobStartOptions, types::JobStatus},
        handlers::{
            cscs_file_download, cscs_file_list, cscs_file_upload, cscs_job_cancel, cscs_job_details, cscs_job_list,
            cscs_job_log, cscs_login, cscs_start_job, cscs_system_list, cscs_system_set,
        },
    },
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
pub(crate) async fn cli_cscs_job_list(system: Option<String>, platform: Option<ComputePlatform>) -> Result<()> {
    match cscs_job_list(system, platform).await {
        Ok(jobs) => {
            let mut table = tabled::Table::new(jobs);
            table.with(tabled::settings::Style::modern());
            println!("{}", table);
            Ok(())
        }
        Err(e) => Err(e),
    }
}
pub(crate) async fn cli_cscs_job_detail(
    job_id: i64,
    system: Option<String>,
    platform: Option<ComputePlatform>,
) -> Result<()> {
    match cscs_job_details(job_id, system, platform).await {
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

pub(crate) async fn cli_cscs_job_log(
    job_id: i64,
    stderr: bool,
    system: Option<String>,
    platform: Option<ComputePlatform>,
) -> Result<()> {
    match cscs_job_log(job_id, stderr, system, platform).await {
        Ok(content) => {
            println!("{}", content);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn cli_cscs_job_start(
    name: Option<String>,
    options: JobStartOptions,
    system: Option<String>,
    platform: Option<ComputePlatform>,
    account: Option<String>,
) -> Result<()> {
    match cscs_start_job(name, options, system, platform, account).await {
        Ok(_) => {
            println!("Job started");
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub(crate) async fn cli_cscs_job_cancel(
    job_id: i64,
    system: Option<String>,
    platform: Option<ComputePlatform>,
) -> Result<()> {
    cscs_job_cancel(job_id, system, platform).await
}

pub(crate) async fn cli_cscs_system_list(platform: Option<ComputePlatform>) -> Result<()> {
    match cscs_system_list(platform).await {
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

pub(crate) async fn cli_cscs_file_list(
    path: PathBuf,
    system: Option<String>,
    platform: Option<ComputePlatform>,
) -> Result<()> {
    match cscs_file_list(path, system, platform).await {
        Ok(path_entries) => {
            let mut table = tabled::Table::new(path_entries);
            table.with(tabled::settings::Style::empty());
            println!("{}", table);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub(crate) async fn cli_cscs_file_download(
    remote: PathBuf,
    local: PathBuf,
    account: Option<String>,
    system: Option<String>,
    platform: Option<ComputePlatform>,
) -> Result<()> {
    let local = if local.is_dir() {
        local.join(remote.file_name().ok_or(eyre!("couldn't get name of remote file"))?)
    } else {
        local
    };
    match cscs_file_download(remote, local.clone(), account, system.clone(), platform.clone()).await {
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
                if let Some(job) = cscs_job_details(job_data.0, system.clone(), platform.clone()).await? {
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

            let mut output = File::create(local).await?;
            let mut stream = reqwest::get(job_data.1).await?.bytes_stream();
            let mut progress = 0;
            let mut start_time = Instant::now();
            while let Some(chunk_result) = stream.next().await {
                let chunk = chunk_result?;
                output.write_all(&chunk).await?;
                progress += chunk.len();

                if start_time.elapsed() >= Duration::from_secs(1) {
                    print!("\rDownloaded {}/{}Mb", progress / 1024 / 1024, job_data.2 / 1024 / 1024);
                    std::io::stdout().flush()?;
                    start_time = Instant::now();
                }
            }
            output.flush().await?;
            println!(); //force newline
            println!("Download complete");

            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub(crate) async fn cli_cscs_file_upload(
    local: PathBuf,
    remote: PathBuf,
    account: Option<String>,
    system: Option<String>,
    platform: Option<ComputePlatform>,
) -> Result<()> {
    match cscs_file_upload(local.clone(), remote, account, system, platform).await {
        Ok(None) => {
            println!("File successfully uploaded");
            Ok(())
        }
        Ok(Some(transfer_data)) => {
            println!("starting file transfer, this might take a while");
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
                    local.clone(),
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
            let body = format!("<CompleteMultipartUpload>{}</CompleteMultipartUpload>", body);
            let req = client.post(transfer_data.1.complete_upload_url).body(body).build()?;
            let resp = client.execute(req).await?;
            match resp.error_for_status() {
                Ok(_) => {
                    println!("done");
                    Ok(())
                }
                Err(e) => Err(e).wrap_err("failed to complete upload"),
            }
        }
        Err(e) => Err(e),
    }
}

async fn upload_chunk(path: PathBuf, offset: u64, size: u64, url: Url) -> Result<String> {
    let client = reqwest::Client::new();

    let source_file = File::open(path).await?;
    let mut buf = vec![];
    let mut reader = BufReader::new(source_file);
    reader.seek(SeekFrom::Start(offset)).await?;
    let mut chunk = reader.take(size);
    chunk.read_to_end(&mut buf).await?;
    let req = client.put(url).body(buf).build()?;
    let resp = client.execute(req).await?;
    Ok(resp.headers()["etag"].to_str()?.to_owned())
}
