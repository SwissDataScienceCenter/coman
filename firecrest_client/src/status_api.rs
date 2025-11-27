use eyre::Result;

use crate::{
    client::FirecrestClient,
    types::{GetSystemsResponse, UserInfoResponse},
};

pub async fn get_status_systems(client: &FirecrestClient) -> Result<GetSystemsResponse> {
    let response = client.get("status/systems", None).await?;
    let model: GetSystemsResponse = serde_json::from_str(response.as_str())?;
    Ok(model)
}
pub async fn get_status_userinfo(client: &FirecrestClient, system_name: &str) -> Result<UserInfoResponse> {
    let response = client.get(&format!("status/{system_name}/userinfo"), None).await?;
    let model: UserInfoResponse = serde_json::from_str(response.as_str())?;
    Ok(model)
}
