use color_eyre::eyre::{Context, Result};
use firecrest_client::{
    client::FirecrestClient,
    compute_api::{get_compute_system_jobs, post_compute_system_job},
    status_api::get_status_systems,
    types::{
        FileSystem as CSCSFileSystem, HealthCheckType, HpcclusterOutput, JobModelOutput,
        SchedulerServiceHealth,
    },
};
use std::fmt::Display;
use strum::Display;
use tabled::Table;

use crate::{
    cscs::handlers::ACCESS_TOKEN_SECRET_NAME,
    trace_dbg,
    util::keyring::{Secret, get_secret},
};
#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, tabled::Tabled)]
struct FileSystem {
    data_type: String,
    default_work_dir: bool,
    path: String,
}
impl From<CSCSFileSystem> for FileSystem {
    fn from(value: CSCSFileSystem) -> Self {
        Self {
            data_type: serde_json::to_string(&value.data_type).expect("got invalid data type"),
            default_work_dir: value.default_work_dir.unwrap_or(false),
            path: value.path,
        }
    }
}

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, Display)]
pub enum JobStatus {
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
                other => panic!("got job status: {}", other),
            },
            user: value.user.unwrap_or("".to_string()),
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
    name: String,
    #[tabled(skip)]
    file_systems: Vec<FileSystem>,
    #[tabled(display = "display_health")]
    services_health: Option<Vec<ServicesHealth>>,
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
    pub async fn start_job(&self, system_name: String) -> Result<()> {
        let result = post_compute_system_job(&self.client, &system_name,"test","#!/bin/bash\n\n#SBATCH --job-name=test\n#SBATCH --ntasks=1\n#SBATCH --time=10:00\n\n sleep 360",None).await?;
        let _ = trace_dbg!(result);

        Ok(())
    }
    pub async fn list_systems(&self) -> Result<Vec<System>> {
        let result = get_status_systems(&self.client)
            .await
            .wrap_err("couldn't list CSCS systems")?;
        Ok(result.systems.into_iter().map(|s| s.into()).collect())
    }
    pub async fn list_jobs(
        &self,
        system_name: String,
        all_users: Option<bool>,
    ) -> Result<Vec<Job>> {
        let result = get_compute_system_jobs(&self.client, &system_name, all_users)
            .await
            .wrap_err("couldn't fetch cscs jobs")?;
        Ok(result
            .jobs
            .map(|jobs| jobs.into_iter().map(|j| j.into()).collect())
            .unwrap_or(vec![]))
    }
}
