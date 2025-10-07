use color_eyre::Report;
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::{
    app::{Mode, SubMode},
    focus_manager::Focus,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub message: String,
    pub full: String,
}

impl ErrorDetail {
    pub fn new(message: &'static str, err: Report) -> Self {
        Self {
            message: message.to_string(),
            full: format!("{:#?}", err.wrap_err(message)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(ErrorDetail),
    Help,
    Menu,
    MenuUp,
    MenuDown,
    Enter,
    Escape,
    Up,
    Down,
    Mode(Mode),
    SubMode(SubMode),
    RemoteRefresh,
    CSCSLogin,
    CSCSToken(String, Option<String>),
    ClosePopup,
    RequestFocus(String, Focus),
    ReleaseFocus(String),
    FocusChanged(String, Focus),
}
