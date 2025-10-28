use tui_realm_stdlib::Label;
use tuirealm::Component;

use crate::app::{messages::Msg, user_events::UserEvent};

#[derive(MockComponent)]
pub struct HelloWorldLabel {
    component: Label,
}

impl HelloWorldLabel {
    pub fn new() -> Self {
        Self {
            component: Label::default().text("Hello World"),
        }
    }
}

impl Component<Msg, UserEvent> for HelloWorldLabel {
    fn on(&mut self, _ev: tuirealm::Event<UserEvent>) -> Option<Msg> {
        None
    }
}
