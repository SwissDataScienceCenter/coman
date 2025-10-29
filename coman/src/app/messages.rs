#[derive(Debug, PartialEq)]
pub enum MenuMsg {
    Opened,
    Closed,
    CSCSLogin,
}
#[derive(Debug, PartialEq)]
pub enum Msg {
    AppClose,
    Menu(MenuMsg),
    CSCSLogin,
    CSCSToken(String, Option<String>),
    None,
}
