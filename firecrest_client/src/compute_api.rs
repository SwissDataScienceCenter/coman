use crate::{
    client::FirecrestClient,
    types::{
        GetJobResponse, GetSystemsResponse, JobDescriptionModel, JobDescriptionModelEnv,
        PostJobSubmissionResponse, PostJobSubmitRequest,
    },
};
use eyre::Result;
use serde_json::json;

pub async fn post_compute_system_job(
    client: &FirecrestClient,
    system_name: &str,
    name: &str,
    script: &str,
    working_dir: Option<&str>,
) -> Result<PostJobSubmissionResponse> {
    let body = PostJobSubmitRequest {
        job: JobDescriptionModel {
            name: Some(name.to_string()),
            script: Some(script.to_string()),
            working_directory: working_dir.unwrap_or("/").to_string(),
            env: Some(JobDescriptionModelEnv::Object(json!({"test":"test"}))),
            ..Default::default()
        },
    };
    let body_json = serde_json::to_string(&body)?;
    let body_json = dbg!(body_json);

    let response = client
        .post(
            format!("compute/{system_name}/jobs").as_str(),
            body_json,
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
