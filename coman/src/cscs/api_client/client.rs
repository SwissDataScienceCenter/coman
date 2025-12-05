use std::{collections::HashMap, path::PathBuf};

use color_eyre::eyre::{Context, Result};
use eyre::eyre;
use firecrest_client::{
    client::FirecrestClient,
    compute_api::{
        cancel_compute_system_job, get_compute_system_job, get_compute_system_job_metadata, get_compute_system_jobs,
        post_compute_system_job,
    },
    filesystem_api::{
        get_filesystem_ops_download, get_filesystem_ops_ls, get_filesystem_ops_stat, get_filesystem_ops_tail,
        post_filesystem_ops_mkdir, post_filesystem_ops_upload, post_filesystem_transfer_download,
        post_filesystem_transfer_upload, put_filesystem_ops_chmod,
    },
    status_api::{get_status_systems, get_status_userinfo},
    types::DownloadFileResponseTransferDirectives,
};
use reqwest::Url;

use crate::{
    config::{ComputePlatform, Config},
    cscs::api_client::types::{FileStat, Job, JobDetail, PathEntry, S3Upload, System, UserInfo},
    trace_dbg,
};

pub struct CscsApi {
    client: FirecrestClient,
}

impl CscsApi {
    pub fn new(token: String, platform: Option<ComputePlatform>) -> Result<Self> {
        let config = Config::new()?;
        let client = FirecrestClient::default()
            .base_path(format!(
                "https://api.cscs.ch/{}/firecrest/v2/",
                platform.unwrap_or(config.cscs.current_platform)
            ))?
            .token(token);
        Ok(Self { client })
    }
    pub async fn start_job(
        &self,
        system_name: &str,
        account: Option<String>,
        name: &str,
        script_path: PathBuf,
        envvars: HashMap<String, String>,
    ) -> Result<()> {
        let workingdir = script_path.clone();
        let workingdir = workingdir.parent();
        let _result = post_compute_system_job(
            &self.client,
            system_name,
            account,
            name,
            None,
            Some(script_path),
            workingdir.map(|p| p.to_path_buf()),
            envvars,
        )
        .await?;

        Ok(())
    }
    pub async fn get_system(&self, system: &str) -> Result<Option<System>> {
        let systems = self.list_systems().await?;
        Ok(systems.into_iter().find(|s| s.name == system))
    }
    pub async fn list_systems(&self) -> Result<Vec<System>> {
        let result = get_status_systems(&self.client)
            .await
            .wrap_err("couldn't list CSCS systems")?;
        Ok(result.systems.into_iter().map(|s| s.into()).collect())
    }
    pub async fn list_jobs(&self, system_name: &str, all_users: Option<bool>) -> Result<Vec<Job>> {
        let result = get_compute_system_jobs(&self.client, system_name, all_users)
            .await
            .wrap_err("couldn't fetch cscs jobs")?;
        Ok(result
            .jobs
            .map(|jobs| jobs.into_iter().map(|j| j.into()).collect())
            .unwrap_or(vec![]))
    }
    pub async fn get_job(&self, system_name: &str, job_id: i64) -> Result<Option<JobDetail>> {
        let jobs = get_compute_system_job(&self.client, system_name, job_id)
            .await
            .wrap_err("couldn't fetch job info")?;
        let job = if let Some(jobs) = jobs.jobs
            && !jobs.is_empty()
        {
            jobs[0].clone()
        } else {
            return Ok(None);
        };
        let job_metadata = get_compute_system_job_metadata(&self.client, system_name, job_id)
            .await
            .wrap_err("couldn't fetch job metadata")?;
        let job_metadata = if let Some(meta) = job_metadata.jobs
            && !meta.is_empty()
        {
            meta[0].clone()
        } else {
            return Ok(None);
        };
        Ok(Some((job, job_metadata).into()))
    }

    pub async fn cancel_job(&self, system_name: &str, job_id: i64) -> Result<()> {
        cancel_compute_system_job(&self.client, system_name, job_id)
            .await
            .wrap_err("couldn't delete job")?;
        Ok(())
    }

    pub async fn mkdir(&self, system_name: &str, path: PathBuf) -> Result<()> {
        let _ = post_filesystem_ops_mkdir(&self.client, system_name, path)
            .await
            .wrap_err("couldn't create directory")?;
        Ok(())
    }
    pub async fn chmod(&self, system_name: &str, path: PathBuf, mode: &str) -> Result<()> {
        let _ = put_filesystem_ops_chmod(&self.client, system_name, path, mode)
            .await
            .wrap_err("couldn't change directory permission")?;
        Ok(())
    }
    pub async fn upload(&self, system_name: &str, path: PathBuf, file: Vec<u8>) -> Result<()> {
        post_filesystem_ops_upload(&self.client, system_name, path, file)
            .await
            .wrap_err("couldn't upload file")?;
        Ok(())
    }
    pub async fn transfer_upload(
        &self,
        system_name: &str,
        account: Option<String>,
        path: PathBuf,
        size: i64,
    ) -> Result<(i64, S3Upload)> {
        let job = post_filesystem_transfer_upload(&self.client, system_name, account, path, size)
            .await
            .wrap_err("couldn't upload file")?;
        if let DownloadFileResponseTransferDirectives::S3(directives) = job.transfer_directives {
            Ok((job.transfer_job.job_id, S3Upload::convert(directives, size as u64)?))
        } else {
            trace_dbg!(job);
            Err(eyre!("didn't get S3 transfer directive"))
        }
    }
    pub async fn download(&self, system_name: &str, path: PathBuf) -> Result<String> {
        let content = get_filesystem_ops_download(&self.client, system_name, path)
            .await
            .wrap_err("couldn't download file")?;
        Ok(content)
    }
    pub async fn transfer_download(
        &self,
        system_name: &str,
        account: Option<String>,
        path: PathBuf,
    ) -> Result<(i64, Url)> {
        let job = post_filesystem_transfer_download(&self.client, system_name, account, path)
            .await
            .wrap_err("couldn't transfer file")?;
        if let DownloadFileResponseTransferDirectives::S3(directives) = job.transfer_directives {
            let download_url = Url::parse(&directives.download_url.unwrap())?;
            Ok((job.transfer_job.job_id, download_url))
        } else {
            Err(eyre!("didn't get S3 transfer directive"))
        }
    }
    pub async fn tail(&self, system_name: &str, path: PathBuf, lines: usize) -> Result<String> {
        let result = get_filesystem_ops_tail(&self.client, system_name, path, lines)
            .await
            .wrap_err("couldn't tail file")?;
        match result.output {
            Some(output) => Ok(output.content),
            None => Ok("".to_string()),
        }
    }
    pub async fn list_path(&self, system_name: &str, path: PathBuf) -> Result<Vec<PathEntry>> {
        let result = get_filesystem_ops_ls(&self.client, system_name, path)
            .await
            .wrap_err("couldn't list path")?;
        match result.output {
            Some(entries) => Ok(entries.into_iter().map(|e| e.into()).collect()),
            None => Ok(vec![]),
        }
    }
    pub async fn stat_path(&self, system_name: &str, path: PathBuf) -> Result<Option<FileStat>> {
        let result = get_filesystem_ops_stat(&self.client, system_name, path)
            .await
            .wrap_err("couldn't stat file")?;
        Ok(result.output.map(|f| f.into()))
    }
    pub async fn get_userinfo(&self, system_name: &str) -> Result<UserInfo> {
        let result = get_status_userinfo(&self.client, system_name)
            .await
            .wrap_err("couldn't load user info")?;
        Ok(result.into())
    }
}
