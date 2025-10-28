#[derive(Debug, PartialEq)]
pub enum Msg {
    AppClose,
    MenuOpened,
    CSCSLogin,
    CSCSToken(String, Option<String>),
    None,
}
