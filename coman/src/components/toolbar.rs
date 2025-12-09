use tui_realm_stdlib::Label;
use tuirealm::{AttrValue, Attribute, Component, Event, MockComponent};

use crate::app::{
    messages::{Msg, View},
    user_events::UserEvent,
};
const WORKLOAD_TOOLTIP: &str = "q: quit, Esc: close/back, l: logs, f: File view, x: menu, tab: switch view, ?: help";
const FILETREE_TOOLTIP: &str = "q: quit, ↑↓: navigate,←→: collapse/expand, x: menu, ?: help";

#[derive(MockComponent)]
pub struct Toolbar {
    component: Label,
    current_view: View,
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            component: Label::default().text(WORKLOAD_TOOLTIP),
            current_view: View::default(),
        }
    }
}

impl Component<Msg, UserEvent> for Toolbar {
    fn on(&mut self, ev: tuirealm::Event<UserEvent>) -> Option<Msg> {
        match ev {
            Event::User(UserEvent::SwitchedToView(view)) => {
                self.current_view = view;
                match self.current_view {
                    View::Workloads => self.attr(Attribute::Text, AttrValue::String(WORKLOAD_TOOLTIP.to_owned())),
                    View::Files => self.attr(Attribute::Text, AttrValue::String(FILETREE_TOOLTIP.to_owned())),
                }
                None
            }
            _ => None,
        }
    }
}
