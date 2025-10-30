#[derive(Debug, PartialEq)]
pub enum MenuMsg {
    Opened,
    Closed,
    CSCSLogin,
}
#[derive(Debug, PartialEq)]
pub enum ErrorPopupMsg {
    Opened(String),
    Closed,
}
#[derive(Debug, PartialEq)]
pub enum Msg {
    AppClose,
    Menu(MenuMsg),
    ErrorPopup(ErrorPopupMsg),
    Error(String),
    CSCSLogin,
    CSCSToken(String, Option<String>),
    None,
}
