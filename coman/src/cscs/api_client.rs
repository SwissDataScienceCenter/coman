use std::{collections::HashMap, path::PathBuf};

use chrono::prelude::*;
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
    types::{
        DownloadFileResponseTransferDirectives, File as CSCSFile, FileStat as CSCSFileStat,
        FileSystem as CSCSFileSystem, FileSystemDataType, HPCCluster, HealthCheckType, JobMetadataModel, JobModel,
        S3TransferResponse, SchedulerServiceHealth, UserInfoResponse,
    },
};
use reqwest::Url;
use strum::Display;

use crate::{
    config::{ComputePlatform, Config},
    trace_dbg,
};

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, tabled::Tabled)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
    pub group: String,
}
impl From<UserInfoResponse> for UserInfo {
    fn from(value: UserInfoResponse) -> Self {
        Self {
            id: value.user.id,
            name: value.user.name,
            group: value.group.name,
        }
    }
}

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, tabled::Tabled, Display)]
pub enum FileSystemType {
    Users,
    Store,
    Archive,
    Apps,
    Scratch,
    Project,
}
impl From<FileSystemDataType> for FileSystemType {
    fn from(value: FileSystemDataType) -> Self {
        match value {
            FileSystemDataType::Users => Self::Users,
            FileSystemDataType::Store => Self::Store,
            FileSystemDataType::Archive => Self::Archive,
            FileSystemDataType::Apps => Self::Apps,
            FileSystemDataType::Scratch => Self::Scratch,
            FileSystemDataType::Project => Self::Project,
        }
    }
}
impl From<String> for FileSystemType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "users" => Self::Users,
            "store" => Self::Store,
            "archive" => Self::Archive,
            "apps" => Self::Apps,
            "scratch" => Self::Scratch,
            "project" => Self::Project,
            _ => panic!("unknown file system type: {}", value),
        }
    }
}
#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, tabled::Tabled)]
pub struct FileSystem {
    pub data_type: FileSystemType,
    pub default_work_dir: bool,
    pub path: String,
}
impl From<CSCSFileSystem> for FileSystem {
    fn from(value: CSCSFileSystem) -> Self {
        Self {
            data_type: value.data_type.into(),
            default_work_dir: value.default_work_dir.unwrap_or(false),
            path: value.path,
        }
    }
}

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, Display)]
pub enum PathType {
    Directory,
    File,
}

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, tabled::Tabled)]
pub struct PathEntry {
    #[tabled(order = 3)]
    pub name: String,
    #[tabled(order = 1)]
    pub path_type: PathType,
    #[tabled(display("display_option"), order = 0)]
    pub permissions: Option<String>,
    #[tabled(display("display_option"), order = 2)]
    pub size: Option<usize>,
}

fn display_option<V>(value: &Option<V>) -> String
where
    V: ToString,
{
    match value {
        Some(s) => s.to_string(),
        None => "".to_owned(),
    }
}

impl From<CSCSFile> for PathEntry {
    fn from(value: CSCSFile) -> Self {
        let size = match value.size.parse::<usize>() {
            Ok(size) => size,
            Err(err) => panic!("Couldn't parse file size {}: {:?}", value.size, err),
        };
        Self {
            name: value.name,
            path_type: match value.r#type.as_str() {
                "d" => PathType::Directory,
                "-" => PathType::File,
                _ => panic!("Unknown file type: {}", value.r#type),
            },
            permissions: Some(value.permissions),
            size: Some(size),
        }
    }
}

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord)]
pub struct FileStat {
    pub size: i64,
}
impl From<CSCSFileStat> for FileStat {
    fn from(value: CSCSFileStat) -> Self {
        Self { size: value.size }
    }
}

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, Display)]
pub enum JobStatus {
    Pending,
    Running,
    Finished,
    Cancelled,
    Failed,
    Timeout,
}
impl From<String> for JobStatus {
    fn from(value: String) -> Self {
        match value.as_str() {
            "RUNNING" => JobStatus::Running,
            "FAILED" => JobStatus::Failed,
            "COMPLETED" => JobStatus::Finished,
            "CANCELLED" => JobStatus::Cancelled,
            "PENDING" => JobStatus::Pending,
            "TIMEOUT" => JobStatus::Timeout,
            other => panic!("got job status: {}", other),
        }
    }
}
#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, tabled::Tabled)]
pub struct Job {
    pub id: usize,
    pub name: String,
    pub status: JobStatus,
    pub user: String,
    #[tabled(display("display_option_datetime"))]
    pub start_date: Option<DateTime<Local>>,
    #[tabled(display("display_option_datetime"))]
    pub end_date: Option<DateTime<Local>>,
}
impl From<JobModel> for Job {
    fn from(value: JobModel) -> Self {
        Self {
            id: value.job_id as usize,
            name: value.name,
            status: value.status.state.into(),
            user: value.user.unwrap_or("".to_string()),
            start_date: value
                .time
                .start
                .map(|s| DateTime::from_timestamp_secs(s).unwrap().with_timezone(&Local)),
            end_date: value
                .time
                .end
                .map(|e| DateTime::from_timestamp_secs(e).unwrap().with_timezone(&Local)),
        }
    }
}

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, tabled::Tabled)]
pub struct JobDetail {
    pub id: usize,
    pub name: String,
    #[tabled(display("display_option_datetime"))]
    pub start_date: Option<DateTime<Local>>,
    #[tabled(display("display_option_datetime"))]
    pub end_date: Option<DateTime<Local>>,
    pub status: JobStatus,
    pub status_reason: String,
    pub exit_code: i64,
    pub user: String,
    pub stdout: String,
    pub stderr: String,
    pub stdin: String,
}
fn display_option_datetime(value: &Option<DateTime<Local>>) -> String {
    match value {
        Some(dt) => dt.to_string(),
        None => "".to_owned(),
    }
}
impl From<(JobModel, JobMetadataModel)> for JobDetail {
    fn from(value: (JobModel, JobMetadataModel)) -> Self {
        Self {
            id: value.0.job_id as usize,
            name: value.0.name,
            start_date: value
                .0
                .time
                .start
                .map(|s| DateTime::from_timestamp_secs(s).unwrap().with_timezone(&Local)),
            end_date: value
                .0
                .time
                .end
                .map(|e| DateTime::from_timestamp_secs(e).unwrap().with_timezone(&Local)),
            status: value.0.status.state.into(),
            status_reason: value.0.status.state_reason.unwrap_or("".to_owned()),
            exit_code: value.0.status.exit_code.unwrap_or(0),
            user: value.0.user.unwrap_or("".to_string()),
            stdout: value.1.standard_output.unwrap_or("".to_owned()),
            stderr: value.1.standard_error.unwrap_or("".to_owned()),
            stdin: value.1.standard_input.unwrap_or("".to_owned()),
        }
    }
}

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, Display, tabled::Tabled)]
pub enum ServiceType {
    Scheduler,
    Filesystem,
    Ssh,
    S3,
    Exception,
}

impl From<HealthCheckType> for ServiceType {
    fn from(value: HealthCheckType) -> Self {
        match value {
            HealthCheckType::Scheduler => ServiceType::Scheduler,
            HealthCheckType::Filesystem => ServiceType::Filesystem,
            HealthCheckType::Ssh => ServiceType::Ssh,
            HealthCheckType::S3 => ServiceType::S3,
            HealthCheckType::Exception => ServiceType::Exception,
        }
    }
}
impl From<String> for ServiceType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "scheduler" => ServiceType::Scheduler,
            "filesystem" => ServiceType::Filesystem,
            "ssh" => ServiceType::Ssh,
            "s3" => ServiceType::S3,
            "exception" => ServiceType::Exception,
            _ => panic!("unknown service type: {}", value),
        }
    }
}

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, tabled::Tabled)]
pub struct ServicesHealth {
    #[tabled(order = 1)]
    healthy: bool,
    #[tabled(order = 0)]
    service_type: ServiceType,

    #[tabled(skip)]
    message: String,
}

impl From<SchedulerServiceHealth> for ServicesHealth {
    fn from(value: SchedulerServiceHealth) -> Self {
        Self {
            healthy: value.healthy.unwrap_or(false),
            service_type: value.service_type.into(),
            message: value.message.unwrap_or("".to_string()),
        }
    }
}

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, tabled::Tabled)]
pub struct System {
    pub name: String,
    #[tabled(skip)]
    pub file_systems: Vec<FileSystem>,
    #[tabled(display = "display_health")]
    pub services_health: Option<Vec<ServicesHealth>>,
}
impl From<HPCCluster> for System {
    fn from(value: HPCCluster) -> Self {
        Self {
            name: value.name,
            file_systems: value
                .file_systems
                .map(|f| f.into_iter().map(|fs| fs.into()).collect())
                .unwrap_or_default(),
            services_health: value
                .services_health
                .map(|s| s.into_iter().map(|sh| sh.into()).collect()),
        }
    }
}
fn display_health(h: &Option<Vec<ServicesHealth>>) -> String {
    h.clone()
        .map(|healths| {
            tabled::Table::new(healths)
                .with(tabled::settings::Style::extended())
                .to_string()
        })
        .unwrap_or("".to_string())
}

#[derive(Debug, Clone)]
pub struct S3Upload {
    pub parts_upload_urls: Vec<Url>,
    pub complete_upload_url: Url,
    pub part_size: u64,
    pub num_parts: u64,
}

impl S3Upload {
    fn convert(value: S3TransferResponse, size: u64) -> Result<Self> {
        let complete_url = value.complete_upload_url.ok_or(eyre!("no upload completion url set"))?;
        let part_urls = value.parts_upload_urls.ok_or(eyre!("no part upload urls set"))?;
        let part_size = value.max_part_size.ok_or(eyre!("couldn't parse size"))? as u64;
        Ok(Self {
            parts_upload_urls: part_urls
                .iter()
                .map(|u| Url::parse(u).wrap_err("couldn't parse url"))
                .collect::<Result<Vec<Url>>>()?,
            part_size,
            complete_upload_url: Url::parse(&complete_url)?,
            num_parts: (size.div_ceil(part_size)),
        })
    }
}

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
