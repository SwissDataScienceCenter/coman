#[derive(Debug, PartialEq)]
pub enum MenuMsg {
    Opened,
    Closed,
    CscsLogin,
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
pub enum CscsMsg {
    Login,
}

#[derive(Debug, PartialEq)]
pub enum Msg {
    AppClose,
    Menu(MenuMsg),
    InfoPopup(InfoPopupMsg),
    ErrorPopup(ErrorPopupMsg),
    Error(String),
    Info(String),
    Cscs(CscsMsg),
    None,
}
