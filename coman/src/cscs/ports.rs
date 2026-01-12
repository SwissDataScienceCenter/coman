use std::{path::PathBuf, time::Duration};

use color_eyre::{
    Section,
    eyre::{Context, Report, Result, eyre},
};
use futures::StreamExt;
use openidconnect::core::CoreDeviceAuthorizationResponse;
use tokio::{fs::File, io::AsyncWriteExt, sync::mpsc, time::Instant};
use tuirealm::{
    Event,
    listener::{ListenerResult, PollAsync},
};

use crate::{
    app::user_events::{CscsEvent, FileEvent, StatusEvent, UserEvent},
    config::Config,
    cscs::{
        api_client::types::{JobStatus, PathEntry, PathType},
        handlers::{
            cscs_file_download, cscs_file_list, cscs_job_cancel, cscs_job_details, cscs_job_list, cscs_job_log,
            cscs_stat_path, cscs_system_list, cscs_user_info,
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
        match cscs_job_list(None, None).await {
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
            match cscs_system_list(None).await {
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

pub enum JobLogAction {
    Job(usize),
    SwitchLog,
    Stop,
}

/// This port handles polling the logs of a CSCS job
pub(crate) struct AsyncJobLogPort {
    receiver: mpsc::Receiver<JobLogAction>,
    current_job: Option<usize>,
    stderr: bool,
}

impl AsyncJobLogPort {
    pub fn new(receiver: mpsc::Receiver<JobLogAction>) -> Self {
        Self {
            receiver,
            current_job: None,
            stderr: false,
        }
    }
}
#[tuirealm::async_trait]
impl PollAsync<UserEvent> for AsyncJobLogPort {
    async fn poll(&mut self) -> ListenerResult<Option<Event<UserEvent>>> {
        if self.receiver.is_closed() {
            return Ok(Some(Event::None));
        }
        if !self.receiver.is_empty()
            && let Some(val) = self.receiver.recv().await
        {
            match val {
                JobLogAction::Job(jobid) => {
                    self.current_job = Some(jobid);
                    self.stderr = false;
                }
                JobLogAction::SwitchLog => {
                    self.stderr = !self.stderr;
                }
                JobLogAction::Stop => {
                    self.current_job = None;
                }
            }
        }
        if let Some(job_id) = self.current_job {
            match cscs_job_log(job_id as i64, self.stderr, None, None).await {
                Ok(log) => Ok(Some(Event::User(UserEvent::Cscs(CscsEvent::GotJobLog(log))))),
                Err(e) => {
                    // if there was an error getting the log, if it's stderr, switch to stdout which should
                    // always exist. If we're on stdout and it doesn't exist, unset log watching to not spam errors
                    if self.stderr {
                        self.stderr = false;
                    } else {
                        self.current_job = None;
                    }
                    Ok(Some(Event::User(UserEvent::Error(format!(
                        "{:?}",
                        Err::<(), Report>(e).wrap_err("couldn't get log")
                    )))))
                }
            }
        } else {
            Ok(Some(Event::None))
        }
    }
}

pub enum BackgroundTask {
    ListPaths(PathBuf),
    DownloadFile(PathBuf, PathBuf),
    GetJobDetails(usize),
    CancelJob(usize),
}

/// This port handles asynchronous file operations on CSCS
pub(crate) struct AsyncBackgroundTaskPort {
    receiver: mpsc::Receiver<BackgroundTask>,
    event_tx: mpsc::Sender<UserEvent>,
}

impl AsyncBackgroundTaskPort {
    pub fn new(receiver: mpsc::Receiver<BackgroundTask>, event_tx: mpsc::Sender<UserEvent>) -> Self {
        Self { receiver, event_tx }
    }
}
async fn list_files(id: PathBuf) -> Result<Option<Event<UserEvent>>> {
    let id_str = id
        .clone()
        .into_os_string()
        .into_string()
        .map_err(|_| eyre!("couldn't convert id to string".to_owned()))?;
    if id_str == "/" {
        // load file system roots
        let config = Config::new().expect("couldn't load config");
        let user_info = cscs_user_info(None, None).await?;
        let systems = cscs_system_list(None).await?;
        let system = systems
            .iter()
            .find(|s| s.name == config.values.cscs.current_system)
            .unwrap_or_else(|| panic!("couldn't get info for system {}", config.values.cscs.current_system));
        // listing big directories fails in the api and we might not actually be allowed to
        // access the roots of the storage.
        // So we try to append the user name to the paths and use that, if it works
        let mut subpaths = vec![];
        for fs in system.file_systems.clone() {
            let entry =
                match cscs_stat_path(PathBuf::from(fs.path.clone()).join(user_info.name.clone()), None, None).await {
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
        Ok(Some(Event::User(UserEvent::File(FileEvent::List(id_str, subpaths)))))
    } else {
        let subpaths = cscs_file_list(id, None, None).await?;
        Ok(Some(Event::User(UserEvent::File(FileEvent::List(id_str, subpaths)))))
    }
}
async fn download_file(
    remote: PathBuf,
    local: PathBuf,
    event_tx: mpsc::Sender<UserEvent>,
) -> Result<Option<Event<UserEvent>>> {
    match cscs_file_download(remote, local.clone(), None, None, None).await {
        Ok(None) => Ok(Some(Event::User(UserEvent::File(FileEvent::DownloadSuccessful)))),
        Ok(Some(job_data)) => {
            // file is large, so we created a transfer job to s3 that we need to wait on
            // then we can download from s3
            // TODO: add status updates once we have some sort of status line update functionality
            let mut transfer_done = false;
            while !transfer_done {
                if let Some(job) = cscs_job_details(job_data.0, None, None).await? {
                    match job.status {
                        JobStatus::Pending | JobStatus::Running => {
                            event_tx
                                .send(UserEvent::Status(StatusEvent::Info(
                                    "waiting for transfer job".to_owned(),
                                )))
                                .await?;
                        }
                        JobStatus::Finished => transfer_done = true,
                        JobStatus::Cancelled | JobStatus::Failed => {
                            return Ok(Some(Event::User(UserEvent::Error(
                                "file download job failed".to_string(),
                            ))));
                        }
                        JobStatus::Timeout => {
                            return Ok(Some(Event::User(UserEvent::Error(
                                "file download job timed out".to_string(),
                            ))));
                        }
                    }
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }

            // download from s3

            let mut output = File::create(local).await?;
            let mut stream = reqwest::get(job_data.1).await?.bytes_stream();
            let mut start_time = Instant::now();
            let mut progress = 0;
            while let Some(chunk_result) = stream.next().await {
                let chunk = chunk_result?;
                output.write_all(&chunk).await?;
                progress += chunk.len();

                if start_time.elapsed() >= Duration::from_millis(500) {
                    event_tx
                        .send(UserEvent::Status(StatusEvent::Progress(
                            "Downloading".to_owned(),
                            100 * progress / job_data.2,
                        )))
                        .await?;
                    start_time = Instant::now();
                }
            }
            output.flush().await?;
            Ok(Some(Event::User(UserEvent::File(FileEvent::DownloadSuccessful))))
        }
        Err(e) => Ok(Some(Event::User(UserEvent::Error(format!(
            "{:?}",
            Err::<(), Report>(e).wrap_err("couldn't download path")
        ))))),
    }
}
#[tuirealm::async_trait]
impl PollAsync<UserEvent> for AsyncBackgroundTaskPort {
    async fn poll(&mut self) -> ListenerResult<Option<Event<UserEvent>>> {
        if self.receiver.is_closed() {
            return Ok(None);
        }
        if let Some(action) = self.receiver.recv().await {
            let event_tx = self.event_tx.clone();
            match action {
                BackgroundTask::ListPaths(id) => match list_files(id).await {
                    Ok(event) => Ok(event),
                    Err(e) => Ok(Some(Event::User(UserEvent::Error(format!(
                        "{:?}",
                        Err::<(), Report>(e).wrap_err("couldn't list subpaths")
                    ))))),
                },
                BackgroundTask::DownloadFile(remote, local) => match download_file(remote, local, event_tx).await {
                    Ok(event) => Ok(event),
                    Err(e) => Ok(Some(Event::User(UserEvent::Error(format!(
                        "{:?}",
                        Err::<(), Report>(e).wrap_err("couldn't download file")
                    ))))),
                },
                BackgroundTask::GetJobDetails(job_id) => match cscs_job_details(job_id as i64, None, None).await {
                    Ok(Some(details)) => Ok(Some(Event::User(UserEvent::Cscs(CscsEvent::GotJobDetails(details))))),
                    Ok(None) => Ok(Some(Event::None)),
                    Err(e) => Ok(Some(Event::User(UserEvent::Error(format!(
                        "{:?}",
                        Err::<(), Report>(e).wrap_err("couldn't get job details")
                    ))))),
                },
                BackgroundTask::CancelJob(job_id) => match cscs_job_cancel(job_id as i64, None, None).await {
                    Ok(()) => Ok(Some(Event::None)),
                    Err(e) => Ok(Some(Event::User(UserEvent::Error(format!(
                        "{:?}",
                        Err::<(), Report>(e).wrap_err("couldn't cancel job")
                    ))))),
                },
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
