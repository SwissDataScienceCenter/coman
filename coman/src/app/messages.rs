use std::path::PathBuf;

use crate::{app::user_events::UserEvent, cscs::api_client::System};

#[derive(Debug, PartialEq)]
pub enum MenuMsg {
    Opened,
    Closed,
    CscsLogin,
    CscsSwitchSystem,
    Event(UserEvent),
}

#[derive(Debug, PartialEq)]
pub enum InfoPopupMsg {
    Opened(String),
    Closed,
}
#[derive(Debug, PartialEq)]
pub enum ErrorPopupMsg {
    Opened(String),
    Closed,
}

#[derive(Debug, PartialEq)]
pub enum LoginPopupMsg {
    Opened,
    Closed,
    LoginDone(String, String),
}
#[derive(Debug, PartialEq)]
pub enum DownloadPopupMsg {
    Opened(PathBuf),
    PathSet(PathBuf, PathBuf),
    Closed,
}
#[derive(Debug, PartialEq)]
pub enum SystemSelectMsg {
    Opened(Vec<System>),
    Closed,
    SystemSelected(String),
}

#[derive(Debug, PartialEq)]
pub enum CscsMsg {
    Login(String, String),
    SelectSystem,
    SystemSelected(String),
}
#[derive(Debug, PartialEq)]
pub enum JobMsg {
    Show(usize),
    Switch,
    Close,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Eq, Ord, strum::Display)]
pub enum View {
    #[default]
    Workloads,
    Files,
}
#[derive(Debug, PartialEq)]
pub enum Msg {
    AppClose,
    Menu(MenuMsg),
    InfoPopup(InfoPopupMsg),
    ErrorPopup(ErrorPopupMsg),
    LoginPopup(LoginPopupMsg),
    DownloadPopup(DownloadPopupMsg),
    SystemSelectPopup(SystemSelectMsg),
    Error(String),
    Info(String),
    Cscs(CscsMsg),
    Job(JobMsg),
    ChangeView(View),
    CreateEvent(UserEvent),
    None,
}
