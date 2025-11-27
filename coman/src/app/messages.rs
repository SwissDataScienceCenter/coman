use crate::cscs::api_client::System;

#[derive(Debug, PartialEq)]
pub enum MenuMsg {
    Opened,
    Closed,
    CscsLogin,
    CscsSwitchSystem,
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
pub enum Msg {
    AppClose,
    Menu(MenuMsg),
    InfoPopup(InfoPopupMsg),
    ErrorPopup(ErrorPopupMsg),
    LoginPopup(LoginPopupMsg),
    SystemSelectPopup(SystemSelectMsg),
    Error(String),
    Info(String),
    Cscs(CscsMsg),
    None,
}
