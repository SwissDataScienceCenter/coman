use tui_realm_stdlib::components::Paragraph;
use tuirealm::{
    component::AppComponent,
    component::Component,
    event::Event,
    event::{Key, KeyEvent},
    props::{BorderType, Borders, Color},
};

use crate::app::{
    messages::{InfoPopupMsg, Msg},
    user_events::UserEvent,
};

#[derive(Component)]
pub struct InfoPopup {
    component: Paragraph,
}

impl InfoPopup {
    pub fn new<S: Into<String>>(msg: S) -> Self {
        Self {
            component: Paragraph::default()
                .borders(Borders::default().modifiers(BorderType::Thick).color(Color::Green))
                .title("Info")
                .text(msg.into()),
        }
    }
}

impl AppComponent<Msg, UserEvent> for InfoPopup {
    fn on(&mut self, ev: &Event<UserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) | Event::Keyboard(KeyEvent { code: Key::Enter, .. }) => {
                Some(Msg::InfoPopup(InfoPopupMsg::Closed))
            }
            _ => None,
        }
    }
}
