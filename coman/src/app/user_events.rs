use std::path::PathBuf;

use crate::{
    app::messages::View,
    cscs::api_client::{Job, PathEntry, System},
};

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord)]
pub enum CscsEvent {
    LoggedIn,
    GotWorkloadData(Vec<Job>),
    GotJobLog(String),
    SelectSystemList(Vec<System>),
}

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord)]
pub enum FileEvent {
    List(String, Vec<PathEntry>), // Id, Subpaths
    DownloadFile(PathBuf),
    DownloadCurrentFile,
}
#[derive(Debug, Eq, Clone, PartialOrd, Ord)]
pub enum UserEvent {
    Cscs(CscsEvent),
    File(FileEvent),
    Error(String),
    Info(String),
    SwitchedToView(View),
}

impl PartialEq for UserEvent {
    fn eq(&self, other: &Self) -> bool {
        matches!((self, other), (UserEvent::Cscs(_), UserEvent::Cscs(_)))
            || matches!((self, other), (UserEvent::Error(_), UserEvent::Error(_)))
            || matches!((self, other), (UserEvent::Info(_), UserEvent::Info(_)))
    }
}
