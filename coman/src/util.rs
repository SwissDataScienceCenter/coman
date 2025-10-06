use std::time::Duration;

use color_eyre::Result;
use openidconnect::{
    AdditionalProviderMetadata, ClientId, DeviceAuthorizationUrl, IssuerUrl,
    OAuth2TokenResponse, ProviderMetadata, Scope,
    core::{
        CoreAuthDisplay, CoreClaimName, CoreClaimType, CoreClient, CoreClientAuthMethod,
        CoreDeviceAuthorizationResponse, CoreGrantType, CoreJsonWebKey,
        CoreJweContentEncryptionAlgorithm, CoreJweKeyManagementAlgorithm, CoreResponseMode,
        CoreResponseType, CoreSubjectIdentifierType,
    },
    reqwest,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::action::Action;

const CSCS_URL: &str = "https://auth.cscs.ch/auth/realms/firecrest-clients";
const CSCS_CLIENT_ID: &str = "67905e6e-8edf-4190-ae47-110f61c833ed";
const TIMEOUT: u64 = 60;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct DeviceEndpointProviderMetadata {
    device_authorization_endpoint: DeviceAuthorizationUrl,
}
impl AdditionalProviderMetadata for DeviceEndpointProviderMetadata {}
type DeviceProviderMetadata = ProviderMetadata<
    DeviceEndpointProviderMetadata,
    CoreAuthDisplay,
    CoreClientAuthMethod,
    CoreClaimName,
    CoreClaimType,
    CoreGrantType,
    CoreJweContentEncryptionAlgorithm,
    CoreJweKeyManagementAlgorithm,
    CoreJsonWebKey,
    CoreResponseMode,
    CoreResponseType,
    CoreSubjectIdentifierType,
>;

pub(crate) async fn cscs_login(action_tx: mpsc::UnboundedSender<Action>) -> Result<()> {
    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build");
    let provider_metadata =
        DeviceProviderMetadata::discover_async(IssuerUrl::new(CSCS_URL.to_string())?, &http_client)
            .await?;
    let device_url = provider_metadata
        .additional_metadata()
        .device_authorization_endpoint
        .clone();
    let client = CoreClient::from_provider_metadata(
        provider_metadata.clone(),
        ClientId::new(CSCS_CLIENT_ID.to_string()),
        None,
    )
    .set_device_authorization_url(device_url.clone())
    .set_auth_type(openidconnect::AuthType::RequestBody);
    let details: CoreDeviceAuthorizationResponse = client
        .exchange_device_code()
        .add_scope(Scope::new("profile".to_string()))
        .request_async(&http_client)
        .await?;
    let verify_url = details
        .verification_uri_complete()
        .map(|u| u.secret().to_owned())
        .expect("couldn't construct the full verification url");

    println!(
        "Please visit {} and authorize this application.",
        verify_url
    );
    open::that(verify_url.clone())
        .or_else(|_| {
            println!("Couldn't open browser, please navigate to {}", verify_url);
            std::io::Result::Ok(())
        })
        .unwrap();
    let token = client
        .exchange_device_access_token(&details)?
        .request_async(
            &http_client,
            tokio::time::sleep,
            Some(Duration::from_secs(TIMEOUT)),
        )
        .await?;
    let access_token = token.access_token().secret().to_owned();
    let refresh_token = token.refresh_token().map(|t| t.secret().to_owned());
    action_tx.send(Action::CSCSToken(access_token, refresh_token))?;
    Ok(())
}
