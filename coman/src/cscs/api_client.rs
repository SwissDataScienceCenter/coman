use color_eyre::eyre::{Context, Result};
use firecrest_client::{
    client::FirecrestClient,
    compute_api::{
        get_compute_system_job, get_compute_system_job_metadata, get_compute_system_jobs,
        post_compute_system_job,
    },
    filesystem_api::{
        post_filesystem_ops_mkdir, post_filesystem_ops_upload, put_filesystem_ops_chmod,
    },
    status_api::{get_status_systems, get_status_userinfo},
    types::{
        FileSystem as CSCSFileSystem, FileSystemDataType, HealthCheckType, HpcclusterOutput,
        JobMetadataModel, JobModelOutput, SchedulerServiceHealth, UserInfoResponse,
    },
};
use std::path::PathBuf;
use strum::Display;

use crate::trace_dbg;

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
pub enum JobStatus {
    Pending,
    Running,
    Finished,
    Cancelled,
    Failed,
}
#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, tabled::Tabled)]
pub struct Job {
    pub id: usize,
    pub name: String,
    pub status: JobStatus,
    pub user: String,
}
impl From<JobModelOutput> for Job {
    fn from(value: JobModelOutput) -> Self {
        Self {
            id: value.job_id as usize,
            name: value.name,
            status: match value.status.state.as_str() {
                "RUNNING" => JobStatus::Running,
                "FAILED" => JobStatus::Failed,
                "COMPLETED" => JobStatus::Finished,
                "CANCELLED" => JobStatus::Cancelled,
                "PENDING" => JobStatus::Pending,
                other => panic!("got job status: {}", other),
            },
            user: value.user.unwrap_or("".to_string()),
        }
    }
}

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, tabled::Tabled)]
pub struct JobDetail {
    pub id: usize,
    pub name: String,
    pub status: JobStatus,
    pub status_reason: String,
    pub exit_code: i64,
    pub user: String,
    pub stdout: String,
    pub stderr: String,
    pub stdin: String,
}
impl From<(JobModelOutput, JobMetadataModel)> for JobDetail {
    fn from(value: (JobModelOutput, JobMetadataModel)) -> Self {
        Self {
            id: value.0.job_id as usize,
            name: value.0.name,
            status: match value.0.status.state.as_str() {
                "RUNNING" => JobStatus::Running,
                "FAILED" => JobStatus::Failed,
                "COMPLETED" => JobStatus::Finished,
                "CANCELLED" => JobStatus::Cancelled,
                other => panic!("got job status: {}", other),
            },
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
impl From<HpcclusterOutput> for System {
    fn from(value: HpcclusterOutput) -> Self {
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

pub struct CscsApi {
    client: FirecrestClient,
}

impl CscsApi {
    pub fn new(token: String) -> Result<Self> {
        let client = FirecrestClient::default()
            .base_path("https://api.cscs.ch/hpc/firecrest/v2/".to_owned())?
            .token(token);
        Ok(Self { client })
    }
    pub async fn start_job(
        &self,
        system_name: &str,
        name: &str,
        script_path: PathBuf,
    ) -> Result<()> {
        let workingdir = script_path.clone();
        let workingdir = workingdir.parent();
        let result = post_compute_system_job(
            &self.client,
            system_name,
            name,
            None,
            Some(script_path),
            workingdir.map(|p| p.to_path_buf()),
        )
        .await?;
        let _ = trace_dbg!(result);

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
    pub async fn get_userinfo(&self, system_name: &str) -> Result<UserInfo> {
        let result = get_status_userinfo(&self.client, system_name)
            .await
            .wrap_err("couldn't load user info")?;
        Ok(result.into())
    }
}
