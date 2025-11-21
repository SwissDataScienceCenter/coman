use tui_realm_stdlib::Label;
use tuirealm::Component;

use crate::app::{messages::Msg, user_events::UserEvent};

#[derive(MockComponent)]
pub struct Toolbar {
    component: Label,
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            component: Label::default()
                .text("q: quit, l: logs, tab/shift+tab: change focus, x: menu, ?: help"),
        }
    }
}

impl Component<Msg, UserEvent> for Toolbar {
    fn on(&mut self, _ev: tuirealm::Event<UserEvent>) -> Option<Msg> {
        None
    }
}
