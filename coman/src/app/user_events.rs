use crate::{
    app::messages::View,
    cscs::api_client::types::{Job, JobDetail, PathEntry, System},
};

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord)]
pub enum CscsEvent {
    LoggedIn,
    GotWorkloadData(Vec<Job>),
    GotJobLog(String),
    GotJobDetails(JobDetail),
    SelectSystemList(Vec<System>),
    SystemSelected(String),
}

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord)]
pub enum FileEvent {
    List(String, Vec<PathEntry>), // Id, Subpaths
    DownloadCurrentFile,
    DownloadSuccessful,
    DeleteCurrentFile,
    DeleteSuccessful(String),
}

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord)]
pub enum StatusEvent {
    Progress(String, usize),
    Info(String),
    Warning(String),
}
#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord)]
pub enum JobEvent {
    Cancel,
}

#[derive(Debug, Eq, Clone, PartialOrd, Ord)]
pub enum UserEvent {
    Cscs(CscsEvent),
    File(FileEvent),
    Error(String),
    Info(String),
    Job(JobEvent),
    Status(StatusEvent),
    SwitchedToView(View),
}

impl PartialEq for UserEvent {
    fn eq(&self, other: &Self) -> bool {
        matches!((self, other), (UserEvent::Cscs(_), UserEvent::Cscs(_)))
            || matches!((self, other), (UserEvent::Error(_), UserEvent::Error(_)))
            || matches!((self, other), (UserEvent::Info(_), UserEvent::Info(_)))
    }
}
