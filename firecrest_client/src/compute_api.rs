use crate::{
    client::FirecrestClient,
    types::{GetJobResponse, GetSystemsResponse},
};
use eyre::Result;

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
