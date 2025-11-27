#![allow(dead_code)]
use std::time::Duration;

use color_eyre::Result;
use openidconnect::{
    AdditionalProviderMetadata, ClientId, ClientSecret, CsrfToken, DeviceAuthorizationUrl, IssuerUrl, Nonce,
    OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier, ProviderMetadata, RedirectUrl, Scope,
    core::{
        CoreAuthDisplay, CoreAuthenticationFlow, CoreClaimName, CoreClaimType, CoreClient, CoreClientAuthMethod,
        CoreDeviceAuthorizationResponse, CoreGrantType, CoreJsonWebKey, CoreJweContentEncryptionAlgorithm,
        CoreJweKeyManagementAlgorithm, CoreProviderMetadata, CoreResponseMode, CoreResponseType,
        CoreSubjectIdentifierType,
    },
    reqwest,
};
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::util::keyring::Secret;

pub const CLIENT_ID_SECRET_NAME: &str = "cscs_client_id";
pub const CLIENT_SECRET_SECRET_NAME: &str = "cscs_secret_id";

pub const ACCESS_TOKEN_SECRET_NAME: &str = "cscs_access_token";
pub const REFRESH_TOKEN_SECRET_NAME: &str = "cscs_refresh_token";

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
pub(crate) async fn start_cscs_device_login() -> Result<(CoreDeviceAuthorizationResponse, String)> {
    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build");
    let provider_metadata =
        DeviceProviderMetadata::discover_async(IssuerUrl::new(CSCS_URL.to_string())?, &http_client).await?;
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
        .add_scope(Scope::new("firecrest".to_string()))
        .add_scope(Scope::new("firecrest-v2".to_string()))
        .request_async(&http_client)
        .await?;
    let verify_url = details
        .verification_uri_complete()
        .map(|u| u.secret().to_owned())
        .expect("couldn't construct the full verification url");
    Ok((details, verify_url))
}

pub(crate) async fn finish_cscs_device_login(
    device_details: CoreDeviceAuthorizationResponse,
) -> Result<(Secret, Option<Secret>)> {
    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build");
    let provider_metadata =
        DeviceProviderMetadata::discover_async(IssuerUrl::new(CSCS_URL.to_string())?, &http_client).await?;
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
    let token = client
        .exchange_device_access_token(&device_details)?
        .request_async(&http_client, tokio::time::sleep, Some(Duration::from_secs(TIMEOUT)))
        .await?;
    let access_token = token.access_token().secret().to_owned();
    let refresh_token = token.refresh_token().map(|t| t.secret().to_owned());
    Ok((Secret::new(access_token), refresh_token.map(Secret::new)))
}
pub(crate) async fn start_cscs_pkce_login() -> Result<(PkceCodeVerifier, Nonce, Url)> {
    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build");
    let provider_metadata =
        CoreProviderMetadata::discover_async(IssuerUrl::new(CSCS_URL.to_string())?, &http_client).await?;
    let client = CoreClient::from_provider_metadata(
        provider_metadata.clone(),
        ClientId::new(CSCS_CLIENT_ID.to_string()),
        None,
    )
    .set_redirect_uri(RedirectUrl::new("http://localhost:54321".to_string())?)
    .set_auth_type(openidconnect::AuthType::RequestBody);
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, _, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("profile".to_string()))
        .add_scope(Scope::new("firecrest".to_string()))
        .add_scope(Scope::new("firecrest-v2".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();
    Ok((pkce_verifier, nonce, auth_url))
}

pub(crate) async fn client_credentials_login(
    client_id: Secret,
    client_secret: Secret,
) -> Result<(Secret, Option<Secret>)> {
    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build");
    let provider_metadata =
        CoreProviderMetadata::discover_async(IssuerUrl::new(CSCS_URL.to_string())?, &http_client).await?;
    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new(client_id.0.to_owned()),
        Some(ClientSecret::new(client_secret.0.to_owned())),
    )
    .set_auth_type(openidconnect::AuthType::RequestBody);

    let token = client
        .exchange_client_credentials()?
        .add_scope(Scope::new("profile".to_string()))
        .add_scope(Scope::new("firecrest".to_string()))
        .add_scope(Scope::new("firecrest-v2".to_string()))
        .request_async(&http_client)
        .await?;
    let access_token = token.access_token().secret().to_owned();
    let refresh_token = token.refresh_token().map(|t| t.secret().to_owned());
    Ok((Secret::new(access_token), refresh_token.map(Secret::new)))
}
