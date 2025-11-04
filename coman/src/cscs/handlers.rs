use color_eyre::{
    Result, Section,
    eyre::{Context, eyre},
};
use eyre::Report;
use openidconnect::{
    AdditionalProviderMetadata, ClientId, DeviceAuthorizationUrl, IssuerUrl, OAuth2TokenResponse,
    ProviderMetadata, Scope,
    core::{
        CoreAuthDisplay, CoreClaimName, CoreClaimType, CoreClient, CoreClientAuthMethod,
        CoreDeviceAuthorizationResponse, CoreGrantType, CoreJsonWebKey,
        CoreJweContentEncryptionAlgorithm, CoreJweKeyManagementAlgorithm, CoreResponseMode,
        CoreResponseType, CoreSubjectIdentifierType,
    },
    reqwest,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;
use tuirealm::{
    Event,
    listener::{ListenerResult, PollAsync},
};

use crate::{
    app::user_events::{CscsEvent, UserEvent},
    config::Config,
    cscs::api_client::{CscsApi, Job, System},
    trace_dbg,
    util::keyring::{Secret, get_secret, store_secret},
};

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
    let token = client
        .exchange_device_access_token(&device_details)?
        .request_async(
            &http_client,
            tokio::time::sleep,
            Some(Duration::from_secs(TIMEOUT)),
        )
        .await?;
    let access_token = token.access_token().secret().to_owned();
    let refresh_token = token.refresh_token().map(|t| t.secret().to_owned());
    Ok((Secret::new(access_token), refresh_token.map(Secret::new)))
}

pub(crate) async fn cscs_login() -> Result<(Secret, Option<Secret>)> {
    let (details, verify_url) = start_cscs_device_login().await?;

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
    finish_cscs_device_login(details).await
}

pub(crate) async fn cli_cscs_login() -> Result<()> {
    match cscs_login().await {
        Ok(result) => {
            store_secret(ACCESS_TOKEN_SECRET_NAME, result.0).await?;
            if let Some(refresh_token) = result.1 {
                store_secret(REFRESH_TOKEN_SECRET_NAME, refresh_token).await?;
            }
            println!("Successfully logged in");
        }
        Err(e) => Err(e).wrap_err("couldn't get acccess token")?,
    };
    Ok(())
}
pub(crate) async fn cli_cscs_job_list() -> Result<()> {
    match cscs_job_list().await {
        Ok(jobs) => {
            let mut table = tabled::Table::new(jobs);
            table.with(tabled::settings::Style::modern());
            println!("{}", table);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub(crate) async fn cli_cscs_system_list() -> Result<()> {
    match cscs_system_list().await {
        Ok(systems) => {
            let mut table = tabled::Table::new(systems);
            table.with(tabled::settings::Style::modern());
            println!("{}", table);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub(crate) struct AsyncDeviceFlowPort {
    receiver: mpsc::Receiver<(CoreDeviceAuthorizationResponse, String)>,
    current_response: Option<CoreDeviceAuthorizationResponse>,
}

impl AsyncDeviceFlowPort {
    pub fn new(receiver: mpsc::Receiver<(CoreDeviceAuthorizationResponse, String)>) -> Self {
        Self {
            receiver,
            current_response: None,
        }
    }
}

///tui-realm bases a lot of logic around Events, which are things that originate from the environment through Ports
/// we implement a custom port for waiting for devicecodeflow responses so we don't need to block the UI while waiting
/// for a login
/// this is a state machine that creates two events, first one that the flow has started with the verification URL the
/// user should navigate to, then one once the flow is finished with the token
#[tuirealm::async_trait]
impl PollAsync<UserEvent> for AsyncDeviceFlowPort {
    async fn poll(&mut self) -> ListenerResult<Option<Event<UserEvent>>> {
        if let Some(details) = self.current_response.clone() {
            trace_dbg!("finishing login");
            match finish_cscs_device_login(details).await {
                Ok(result) => {
                    if let Err(e) = store_secret(ACCESS_TOKEN_SECRET_NAME, result.0).await {
                        return Ok(Some(Event::User(UserEvent::Error(format!(
                            "{:?}",
                            Err::<(), Report>(e).wrap_err("couldn't save access token")
                        )))));
                    }
                    if let Some(refresh_token) = result.1
                        && let Err(e) = store_secret(REFRESH_TOKEN_SECRET_NAME, refresh_token).await
                    {
                        return Ok(Some(Event::User(UserEvent::Error(format!(
                            "{:?}",
                            Err::<(), Report>(e).wrap_err("couldn't save refresh token")
                        )))));
                    }
                    self.current_response = None;
                    Ok(Some(Event::User(UserEvent::Cscs(CscsEvent::LoggedIn))))
                }
                Err(e) => {
                    self.current_response = None;
                    Ok(Some(Event::User(UserEvent::Error(format!(
                        "{:?}",
                        Err::<(), Report>(e)
                            .wrap_err("couldn't get access token")
                            .suggestion("please try again")
                    )))))
                }
            }
        } else if let Some((details, url)) = self.receiver.recv().await {
            trace_dbg!("redirecting to url");
            self.current_response = Some(details);
            open::that(url.clone())
                .or_else(|_| {
                    println!("Couldn't open browser, please navigate to {}", url.clone());
                    std::io::Result::Ok(())
                })
                .unwrap();
            Ok(Some(Event::User(UserEvent::Info(format!(
                "Please visit {} and authorize this application.",
                url
            )))))
        } else {
            Ok(None)
        }
    }
}
pub(crate) struct AsyncFetchWorkloadsPort {}

impl AsyncFetchWorkloadsPort {
    pub fn new() -> Self {
        Self {}
    }
}

async fn cscs_system_list() -> Result<Vec<System>> {
    match get_secret(ACCESS_TOKEN_SECRET_NAME).await {
        Ok(Some(access_token)) => {
            let api_client = CscsApi::new(access_token.0).unwrap();
            api_client.list_systems().await
        }
        Ok(None) => Err(eyre!("not logged in")),
        Err(e) => Err(e),
    }
}

async fn cscs_job_list() -> Result<Vec<Job>> {
    match get_secret(ACCESS_TOKEN_SECRET_NAME).await {
        Ok(Some(access_token)) => {
            let api_client = CscsApi::new(access_token.0).unwrap();
            let config = Config::new().unwrap();
            api_client.list_jobs(config.cscs.system, Some(true)).await
        }
        Ok(None) => Err(eyre!("not logged in")),
        Err(e) => Err(e),
    }
}

#[tuirealm::async_trait]
impl PollAsync<UserEvent> for AsyncFetchWorkloadsPort {
    async fn poll(&mut self) -> ListenerResult<Option<Event<UserEvent>>> {
        match cscs_job_list().await {
            Ok(jobs) => {
                let jobs = trace_dbg!(jobs);
                Ok(Some(Event::User(UserEvent::Cscs(
                    CscsEvent::GotWorkloadData(jobs),
                ))))
            }
            Err(e) => {
                trace_dbg!(e);
                Ok(Some(Event::None))
            }
        }
    }
}
