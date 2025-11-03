use color_eyre::eyre::{Context, Result};
use firecrest_client::{
    client::FirecrestClient,
    compute_api::get_compute_system_jobs,
    status_api::get_status_systems,
    types::{FileSystem as CSCSFileSystem, HpcclusterOutput, JobModelOutput},
};
use std::fmt::Display;
use strum::Display;

use crate::{
    cscs::handlers::ACCESS_TOKEN_SECRET_NAME,
    util::keyring::{Secret, get_secret},
};
#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord)]
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
enum JobStatus {
    Running,
    Finished,
    Failed,
}
#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, tabled::Tabled)]
pub struct Job {
    id: usize,
    name: String,
    status: JobStatus,
    user: String,
}
impl From<JobModelOutput> for Job {
    fn from(value: JobModelOutput) -> Self {
        Self {
            id: value.job_id as usize,
            name: value.name,
            status: JobStatus::Running,
            user: value.user.unwrap_or("".to_string()),
        }
    }
}
#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord)]
pub struct System {
    name: String,
    file_systems: Vec<FileSystem>,
}
impl From<HpcclusterOutput> for System {
    fn from(value: HpcclusterOutput) -> Self {
        Self {
            name: value.name,
            file_systems: value
                .file_systems
                .map(|f| f.into_iter().map(|fs| fs.into()).collect())
                .unwrap_or_default(),
        }
    }
}

pub(crate) trait ApiClient {
    async fn start_job(&self) -> Result<()>;
    async fn list_systems(&self) -> Result<Vec<System>>;
    async fn list_jobs(&self, system_name: String, all_users: Option<bool>) -> Result<Vec<Job>>;
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
}

impl ApiClient for CscsApi {
    async fn start_job(&self) -> Result<()> {
        Ok(())
    }
    async fn list_systems(&self) -> Result<Vec<System>> {
        let result = get_status_systems(&self.client)
            .await
            .wrap_err("couldn't list CSCS systems")?;
        Ok(result.systems.into_iter().map(|s| s.into()).collect())
    }
    async fn list_jobs(&self, system_name: String, all_users: Option<bool>) -> Result<Vec<Job>> {
        let result = get_compute_system_jobs(&self.client, system_name.as_str(), None)
            .await
            .wrap_err("couldn't fetch cscs jobs")?;
        Ok(result
            .jobs
            .map(|jobs| jobs.into_iter().map(|j| j.into()).collect())
            .unwrap_or(vec![]))
    }
}
