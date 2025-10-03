use serde::{Deserialize, Serialize};
use strum::Display;

use crate::app::{Mode, SubMode};

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(String),
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
}
