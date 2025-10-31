use color_eyre::eyre::{Context, Result};
use openapi_client::{
    apis::{
        compute_api::{
            get_jobs_compute_system_name_jobs_get, post_job_submit_compute_system_name_jobs_post,
        },
        configuration::Configuration,
        status_api::get_systems_status_systems_get,
    },
    models::{
        FileSystem as CSCSFileSystem, HpcCluster, JobDescriptionModel, JobModel,
        PostJobSubmitRequest,
    },
};

use crate::util::keyring::Secret;
#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord)]
struct FileSystem {
    data_type: String,
    default_work_dir: bool,
    path: String,
}
impl From<CSCSFileSystem> for FileSystem {
    fn from(value: CSCSFileSystem) -> Self {
        Self {
            data_type: value.data_type.to_string(),
            default_work_dir: value.default_work_dir.unwrap_or(false),
            path: value.path,
        }
    }
}

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord)]
enum JobStatus {
    Running,
    Finished,
    Failed,
}
#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord)]
pub struct Job {
    id: usize,
    name: String,
    status: JobStatus,
    user: String,
}
impl From<JobModel> for Job {
    fn from(value: JobModel) -> Self {
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
impl From<HpcCluster> for System {
    fn from(value: HpcCluster) -> Self {
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
    config: Configuration,
    refresh_token: Option<Secret>,
}

impl CscsApi {
    pub fn new(access_token: Secret, refresh_token: Option<Secret>) -> Result<Self> {
        let config = Configuration {
            oauth_access_token: Some(access_token.0),
            base_path: "https://api.cscs.ch/hpc/firecrest/v2".to_owned(),
            ..Default::default()
        };
        Ok(Self {
            config,
            refresh_token,
        })
    }
}

impl ApiClient for CscsApi {
    async fn start_job(&self) -> Result<()> {
        let req = PostJobSubmitRequest::new(JobDescriptionModel::new("/".to_string()));
        let _ = post_job_submit_compute_system_name_jobs_post(&self.config, "system_name", req)
            .await
            .wrap_err("couldn't post job")?;
        Ok(())
    }
    async fn list_systems(&self) -> Result<Vec<System>> {
        let result = get_systems_status_systems_get(&self.config)
            .await
            .wrap_err("couldn't list CSCS systems")?;
        Ok(result.systems.into_iter().map(|s| s.into()).collect())
    }
    async fn list_jobs(&self, system_name: String, all_users: Option<bool>) -> Result<Vec<Job>> {
        let result =
            get_jobs_compute_system_name_jobs_get(&self.config, system_name.as_str(), all_users)
                .await
                .wrap_err("couldn't fetch cscs jobs")?;
        Ok(result
            .jobs
            .flatten()
            .map(|jobs| jobs.into_iter().map(|j| j.into()).collect())
            .unwrap_or(vec![]))
    }
}
