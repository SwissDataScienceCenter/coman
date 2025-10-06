use color_eyre::eyre::{Context, Result};
use openapi_client::{
    apis::{
        compute_api::post_job_submit_compute_system_name_jobs_post, configuration::Configuration,
    },
    models::{JobDescriptionModel, PostJobSubmitRequest},
};

trait ApiClient {
    async fn start_job(&self) -> Result<()>;
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
}
