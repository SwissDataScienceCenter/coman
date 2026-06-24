use tui_realm_stdlib::components::Label;
use tuirealm::{
    component::{AppComponent, Component},
    event::Event,
    props::{AttrValue, Attribute},
};

use crate::app::{
    messages::{Msg, View},
    user_events::UserEvent,
};
const WORKLOAD_TOOLTIP: &str =
    "q: quit, Esc: close/back, Enter: details, l: logs, f: file view, x: menu, tab: switch view";
const FILETREE_TOOLTIP: &str = "q: quit, ↑↓: navigate,←→: collapse/expand, w: workload view x: menu";

#[derive(Component)]
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

impl AppComponent<Msg, UserEvent> for Toolbar {
    fn on(&mut self, ev: &Event<UserEvent>) -> Option<Msg> {
        match ev {
            Event::User(UserEvent::SwitchedToView(view)) => {
                self.current_view = view.to_owned();
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
