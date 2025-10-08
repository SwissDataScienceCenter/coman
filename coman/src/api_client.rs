use color_eyre::eyre::{Context, Result};
use openapi_client::{
    apis::{
        compute_api::post_job_submit_compute_system_name_jobs_post, configuration::Configuration,
        status_api::get_systems_status_systems_get,
    },
    models::{FileSystem as CSCSFileSystem, HpcCluster, JobDescriptionModel, PostJobSubmitRequest},
};
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

struct System {
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

trait ApiClient {
    async fn start_job(&self) -> Result<()>;
    async fn list_systems(&self) -> Result<Vec<System>>;
}

struct CSCSApi {
    config: Configuration,
    refresh_token: Option<String>,
}

impl CSCSApi {
    fn new() -> Result<Self> {
        let access_entry = keyring::Entry::new("coman", "cscs_access_token")?;
        let access_token = access_entry
            .get_password()
            .wrap_err("Failed to read CSCS access token, did you log in?")?;
        let refresh_entry = keyring::Entry::new("coman", "cscs_refresh_token")?;
        let refresh_token = match refresh_entry.get_password() {
            Ok(r) => Some(r),
            Err(e) => match e {
                keyring::Error::NoEntry => None,
                _ => return Err(e).wrap_err("Couldn't get refresh token for CSCS"),
            },
        };
        let config = Configuration {
            oauth_access_token: Some(access_token),

            ..Default::default()
        };
        Ok(Self {
            config,
            refresh_token,
        })
    }
}

impl ApiClient for CSCSApi {
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
}
