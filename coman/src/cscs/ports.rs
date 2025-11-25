use std::path::PathBuf;

use color_eyre::{Section, eyre::Context};
use eyre::Report;
use openidconnect::core::CoreDeviceAuthorizationResponse;
use tokio::sync::mpsc;
use tuirealm::{
    Event,
    listener::{ListenerResult, PollAsync},
};

use crate::{
    app::user_events::{CscsEvent, FileEvent, UserEvent},
    config::Config,
    cscs::{
        api_client::{PathEntry, PathType},
        handlers::{
            cscs_download_path, cscs_file_list, cscs_job_list, cscs_job_log, cscs_stat_path, cscs_system_list,
            cscs_user_info,
        },
        oauth2::{ACCESS_TOKEN_SECRET_NAME, REFRESH_TOKEN_SECRET_NAME, finish_cscs_device_login},
    },
    trace_dbg,
    util::keyring::store_secret,
};

/// This port does the polling of the token for finishing the device code oauth2 flow
#[allow(dead_code)]
pub(crate) struct AsyncDeviceFlowPort {
    receiver: mpsc::Receiver<(CoreDeviceAuthorizationResponse, String)>,
    current_response: Option<CoreDeviceAuthorizationResponse>,
}

impl AsyncDeviceFlowPort {
    #[allow(dead_code)]
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

/// This port periodically fetches jobs from CSCS
pub(crate) struct AsyncFetchWorkloadsPort {}

impl AsyncFetchWorkloadsPort {
    pub fn new() -> Self {
        Self {}
    }
}

#[tuirealm::async_trait]
impl PollAsync<UserEvent> for AsyncFetchWorkloadsPort {
    async fn poll(&mut self) -> ListenerResult<Option<Event<UserEvent>>> {
        match cscs_job_list().await {
            Ok(jobs) => Ok(Some(Event::User(UserEvent::Cscs(CscsEvent::GotWorkloadData(jobs))))),
            Err(e) => {
                let _ = trace_dbg!(e);
                Ok(Some(Event::User(UserEvent::Cscs(CscsEvent::GotWorkloadData(vec![])))))
            }
        }
    }
}

/// This port handles getting available compute systems from CSCS
pub(crate) struct AsyncSelectSystemPort {
    receiver: mpsc::Receiver<()>,
}

impl AsyncSelectSystemPort {
    pub fn new(receiver: mpsc::Receiver<()>) -> Self {
        Self { receiver }
    }
}

#[tuirealm::async_trait]
impl PollAsync<UserEvent> for AsyncSelectSystemPort {
    async fn poll(&mut self) -> ListenerResult<Option<Event<UserEvent>>> {
        if self.receiver.recv().await.is_some() {
            match cscs_system_list().await {
                Ok(systems) => Ok(Some(Event::User(UserEvent::Cscs(CscsEvent::SelectSystemList(systems))))),
                Err(e) => Ok(Some(Event::User(UserEvent::Error(format!(
                    "{:?}",
                    Err::<(), Report>(e)
                        .wrap_err("couldn't get available systems")
                        .suggestion("are you logged in?")
                ))))),
            }
        } else {
            Ok(None)
        }
    }
}

/// This port handles polling the logs of a CSCS job
pub(crate) struct AsyncJobLogPort {
    receiver: mpsc::Receiver<Option<usize>>,
    current_job: Option<usize>,
}

impl AsyncJobLogPort {
    pub fn new(receiver: mpsc::Receiver<Option<usize>>) -> Self {
        Self {
            receiver,
            current_job: None,
        }
    }
}
#[tuirealm::async_trait]
impl PollAsync<UserEvent> for AsyncJobLogPort {
    async fn poll(&mut self) -> ListenerResult<Option<Event<UserEvent>>> {
        if self.receiver.is_closed() {
            return Ok(Some(Event::None));
        }
        if !self.receiver.is_empty() {
            if let Some(val) = self.receiver.recv().await {
                self.current_job = val;
            }
            Ok(Some(Event::None))
        } else if let Some(job_id) = self.current_job {
            match cscs_job_log(job_id as i64).await {
                Ok(log) => {
                    let log = trace_dbg!(log);
                    Ok(Some(Event::User(UserEvent::Cscs(CscsEvent::GotJobLog(log)))))
                }
                Err(e) => Ok(Some(Event::User(UserEvent::Error(format!(
                    "{:?}",
                    Err::<(), Report>(e).wrap_err("couldn't get log")
                ))))),
            }
        } else {
            trace_dbg!("nothing");
            Ok(Some(Event::None))
        }
    }
}

pub enum TreeAction {
    List(String),
    Download(String),
}

/// This port handles asynchronous file operations on CSCS
pub(crate) struct AsyncFileTreePort {
    receiver: mpsc::Receiver<TreeAction>,
}

impl AsyncFileTreePort {
    pub fn new(receiver: mpsc::Receiver<TreeAction>) -> Self {
        Self { receiver }
    }
}
#[tuirealm::async_trait]
impl PollAsync<UserEvent> for AsyncFileTreePort {
    async fn poll(&mut self) -> ListenerResult<Option<Event<UserEvent>>> {
        if self.receiver.is_closed() {
            return Ok(None);
        }
        if let Some(action) = self.receiver.recv().await {
            match action {
                TreeAction::List(id) => {
                    if id == "/" {
                        // load file system roots
                        let config = Config::new().expect("couldn't load config");
                        let user_info = match cscs_user_info().await {
                            Ok(user_info) => user_info,
                            Err(e) => {
                                return Ok(Some(Event::User(UserEvent::Error(format!(
                                    "{:?}",
                                    Err::<(), Report>(e).wrap_err("couldn't list path")
                                )))));
                            }
                        };
                        match cscs_system_list().await {
                            Ok(systems) => {
                                let system = systems
                                    .iter()
                                    .find(|s| s.name == config.cscs.current_system)
                                    .unwrap_or_else(|| {
                                        panic!("couldn't get info for system {}", config.cscs.current_system)
                                    });
                                // listing big directories fails in the api and we might not actually be allowed to
                                // access the roots of the storage.
                                // So we try to append the user name to the paths and use that, if it works
                                let mut subpaths = vec![];
                                for fs in system.file_systems.clone() {
                                    let entry = match cscs_stat_path(
                                        PathBuf::from(fs.path.clone()).join(user_info.name.clone()),
                                    )
                                    .await
                                    {
                                        Ok(Some(_)) => PathEntry {
                                            name: format!("{}/{}", fs.path.clone(), user_info.name),
                                            path_type: PathType::Directory,
                                            permissions: None,
                                            size: None,
                                        },
                                        _ => PathEntry {
                                            name: fs.path.clone(),
                                            path_type: PathType::Directory,
                                            permissions: None,
                                            size: None,
                                        },
                                    };
                                    subpaths.push(entry);
                                }
                                Ok(Some(Event::User(UserEvent::File(FileEvent::List(id, subpaths)))))
                            }
                            Err(e) => Ok(Some(Event::User(UserEvent::Error(format!(
                                "{:?}",
                                Err::<(), Report>(e).wrap_err("couldn't list path")
                            ))))),
                        }
                    } else {
                        let id = trace_dbg!(id);
                        let path = PathBuf::from(id.clone());
                        match cscs_file_list(path).await {
                            Ok(subpaths) => {
                                let subpaths = trace_dbg!(subpaths);
                                Ok(Some(Event::User(UserEvent::File(FileEvent::List(id, subpaths)))))
                            }
                            Err(e) => Ok(Some(Event::User(UserEvent::Error(format!(
                                "{:?}",
                                Err::<(), Report>(e).wrap_err("couldn't list path")
                            ))))),
                        }
                    }
                }
                TreeAction::Download(path) => {
                    let path = trace_dbg!(path);
                    let path = PathBuf::from(path);
                    match cscs_download_path(path).await {
                        Ok(content) => {
                            let content = trace_dbg!(content);
                            std::fs::write("/tmp/download.txt", content)
                                .wrap_err("couldn't download file")
                                .unwrap();
                            Ok(Some(Event::None))
                        }
                        Err(e) => Ok(Some(Event::User(UserEvent::Error(format!(
                            "{:?}",
                            Err::<(), Report>(e).wrap_err("couldn't download path")
                        ))))),
                    }
                }
            }
        } else {
            return Ok(Some(Event::None));
        }
    }
}

/// This is a convenience class to create new user events from the model
pub(crate) struct AsyncUserEventPort {
    receiver: mpsc::Receiver<UserEvent>,
}

impl AsyncUserEventPort {
    pub fn new(receiver: mpsc::Receiver<UserEvent>) -> Self {
        Self { receiver }
    }
}
#[tuirealm::async_trait]
impl PollAsync<UserEvent> for AsyncUserEventPort {
    async fn poll(&mut self) -> ListenerResult<Option<Event<UserEvent>>> {
        if let Some(event) = self.receiver.recv().await {
            let event = trace_dbg!(event);
            Ok(Some(Event::User(event)))
        } else {
            Ok(Some(Event::None))
        }
    }
}
