use std::path::PathBuf;

use crate::{
    client::FirecrestClient,
    types::{
        GetJobMetadataResponse, GetJobResponse, JobDescriptionModel, JobDescriptionModelEnv,
        PostJobSubmissionResponse, PostJobSubmitRequest,
    },
};
use eyre::{Result, eyre};
use serde_json::json;

pub async fn post_compute_system_job(
    client: &FirecrestClient,
    system_name: &str,
    name: &str,
    script: Option<&str>,
    script_path: Option<PathBuf>,
    working_dir: Option<PathBuf>,
) -> Result<PostJobSubmissionResponse> {
    if script.is_none() && script_path.is_none() {
        return Err(eyre!("either script or script_path must be set"));
    }
    let body = PostJobSubmitRequest {
        job: JobDescriptionModel {
            name: Some(name.to_string()),
            script: script.map(|s| s.to_owned()),
            script_path: script_path
                .map(|s| s.into_os_string().into_string())
                .transpose()
                .map_err(|_| eyre!("couldn't convert script path"))?,
            working_directory: working_dir
                .map(|s| s.into_os_string().into_string())
                .transpose()
                .map_err(|_| eyre!("couldn't convert working dir path"))?
                .unwrap_or("/".to_owned()),
            env: Some(JobDescriptionModelEnv::Object(json!({"test":"test"}))),
            ..Default::default()
        },
    };
    let body_json = serde_json::to_string(&body)?;
    let response = client
        .post(
            format!("compute/{system_name}/jobs").as_str(),
            body_json,
            None,
            None,
        )
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
        .get(
            format!("compute/{system_name}/jobs/{job_id}").as_str(),
            None,
        )
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
        .get(
            format!("compute/{system_name}/jobs/{job_id}/metadata").as_str(),
            None,
        )
        .await?;
    let model: GetJobMetadataResponse = serde_json::from_str(response.as_str())?;
    Ok(model)
}
