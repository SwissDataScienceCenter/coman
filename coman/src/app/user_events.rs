use crate::cscs::api_client::Job;

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord)]
pub enum CscsEvent {
    LoggedIn,
    GotWorkloadData(Vec<Job>),
}

#[derive(Debug, Eq, Clone, PartialOrd, Ord)]
pub enum UserEvent {
    Cscs(CscsEvent),
    Error(String),
    Info(String),
    None, // this is mainly used to return a nop result that keeps a port alive, as returning no Event stops the port
}

impl PartialEq for UserEvent {
    fn eq(&self, other: &Self) -> bool {
        matches!((self, other), (UserEvent::Cscs(_), UserEvent::Cscs(_)))
            || matches!((self, other), (UserEvent::Error(_), UserEvent::Error(_)))
            || matches!((self, other), (UserEvent::Info(_), UserEvent::Info(_)))
    }
}
