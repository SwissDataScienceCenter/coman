#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord)]
pub struct CscsWorkloadData {}

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord)]
pub enum CscsEvent {
    LoggedIn,
    GotWorkloadData(CscsWorkloadData),
}

#[derive(Debug, Eq, Clone, PartialOrd, Ord)]
pub enum UserEvent {
    Cscs(CscsEvent),
    Error(String),
    Info(String),
}

impl PartialEq for UserEvent {
    fn eq(&self, other: &Self) -> bool {
        matches!((self, other), (UserEvent::Cscs(_), UserEvent::Cscs(_)))
            || matches!((self, other), (UserEvent::Error(_), UserEvent::Error(_)))
            || matches!((self, other), (UserEvent::Info(_), UserEvent::Info(_)))
    }
}
