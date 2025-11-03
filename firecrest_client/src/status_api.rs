use crate::{client::FirecrestClient, types::GetSystemsResponse};
use eyre::Result;

pub async fn get_status_systems(client: &FirecrestClient) -> Result<GetSystemsResponse> {
    let response = client.get("status/systems", None).await?;
    let model: GetSystemsResponse = serde_json::from_str(response.as_str())?;
    Ok(model)
}
