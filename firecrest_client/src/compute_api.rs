use std::{collections::HashMap, path::PathBuf};

use eyre::{Result, WrapErr, eyre};
use serde_json::json;

use crate::{
    client::FirecrestClient,
    types::{
        GetJobMetadataResponse, GetJobResponse, JobDescriptionModel, JobDescriptionModelEnv, PostJobSubmissionResponse,
        PostJobSubmitRequest,
    },
};

#[derive(Debug, Clone, Default)]
pub struct JobOptions<'a> {
    pub script: Option<&'a str>,
    pub script_path: Option<PathBuf>,
    pub working_dir: Option<PathBuf>,
    pub envvars: HashMap<String, String>,
    pub stdout: Option<PathBuf>,
    pub stderr: Option<PathBuf>,
}

pub async fn post_compute_system_job<'a>(
    client: &FirecrestClient,
    system_name: &str,
    account: Option<String>,
    name: &str,
    options: JobOptions<'a>,
) -> Result<PostJobSubmissionResponse> {
    if options.script.is_none() && options.script_path.is_none() {
        return Err(eyre!("either script or script_path must be set"));
    }
    let body = PostJobSubmitRequest {
        job: JobDescriptionModel {
            name: Some(name.to_string()),
            script: options.script.map(|s| s.to_owned()),
            script_path: options
                .script_path
                .map(|s| s.into_os_string().into_string())
                .transpose()
                .map_err(|_| eyre!("couldn't convert script path"))?,
            working_directory: options
                .working_dir
                .map(|s| s.into_os_string().into_string())
                .transpose()
                .map_err(|_| eyre!("couldn't convert working dir path"))?
                .unwrap_or("/".to_owned()),
            env: Some(JobDescriptionModelEnv::Object(json!(options.envvars))),
            standard_output: options
                .stdout
                .map(|p| p.into_os_string().into_string())
                .transpose()
                .map_err(|e| eyre!("Path:{}", e.display()))
                .wrap_err("stdout is not a valid path")?,
            standard_error: options
                .stderr
                .map(|p| p.into_os_string().into_string())
                .transpose()
                .map_err(|e| eyre!("Path:{}", e.display()))
                .wrap_err("stderr is not a valid path")?,
            account,
            ..Default::default()
        },
    };
    let body_json = serde_json::to_string(&body)?;
    let response = client
        .post(format!("compute/{system_name}/jobs").as_str(), body_json, None, None)
        .await?;
    let model: PostJobSubmissionResponse = serde_json::from_str(response.as_str())?;

    Ok(model)
}

pub async fn get_compute_system_jobs(
    client: &FirecrestClient,
    system_name: &str,
    all_users: Option<bool>,
) -> Result<GetJobResponse> {
    let query = match all_users {
        Some(v) => Some(vec![(
            "all_users",
            match v {
                true => "true",
                false => "false",
            },
        )]),
        None => None,
    };
    let response = client
        .get(format!("compute/{system_name}/jobs").as_str(), query)
        .await?;
    let model: GetJobResponse = serde_json::from_str(response.as_str())?;
    Ok(model)
}

pub async fn get_compute_system_job(
    client: &FirecrestClient,
    system_name: &str,
    job_id: i64,
) -> Result<GetJobResponse> {
    let response = client
        .get(format!("compute/{system_name}/jobs/{job_id}").as_str(), None)
        .await?;
    let model: GetJobResponse = serde_json::from_str(response.as_str())?;
    Ok(model)
}
pub async fn get_compute_system_job_metadata(
    client: &FirecrestClient,
    system_name: &str,
    job_id: i64,
) -> Result<GetJobMetadataResponse> {
    let response = client
        .get(format!("compute/{system_name}/jobs/{job_id}/metadata").as_str(), None)
        .await?;
    let model: GetJobMetadataResponse = serde_json::from_str(response.as_str())?;
    Ok(model)
}

pub async fn cancel_compute_system_job(client: &FirecrestClient, system_name: &str, job_id: i64) -> Result<()> {
    let _ = client
        .delete(format!("compute/{system_name}/jobs/{job_id}").as_str(), None)
        .await?;
    Ok(())
}
