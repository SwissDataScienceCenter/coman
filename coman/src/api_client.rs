use color_eyre::eyre::Result;
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
}

impl CSCSApi {
    fn new() -> Self {
        Self {
            config: Configuration::new(),
        }
    }
}

impl ApiClient for CSCSApi {
    async fn start_job(&self) -> Result<()> {
        let req = PostJobSubmitRequest::new(JobDescriptionModel::new("/".to_string()));
        post_job_submit_compute_system_name_jobs_post(&self.config, "system_name", req).await;
        Ok(())
    }
}
