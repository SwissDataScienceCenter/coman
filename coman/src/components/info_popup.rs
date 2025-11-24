use tui_realm_stdlib::Paragraph;
use tuirealm::{
    Component, Event,
    event::{Key, KeyEvent},
    props::{Alignment, BorderType, Borders, Color, TextSpan},
};

use crate::app::{
    messages::{InfoPopupMsg, Msg},
    user_events::UserEvent,
};

#[derive(MockComponent)]
pub struct InfoPopup {
    component: Paragraph,
}

impl InfoPopup {
    pub fn new<S: Into<String>>(msg: S) -> Self {
        Self {
            component: Paragraph::default()
                .borders(Borders::default().modifiers(BorderType::Thick).color(Color::Green))
                .title("Info", Alignment::Left)
                .text(vec![TextSpan::from(msg)]),
        }
    }
}

impl Component<Msg, UserEvent> for InfoPopup {
    fn on(&mut self, ev: tuirealm::Event<UserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) | Event::Keyboard(KeyEvent { code: Key::Enter, .. }) => {
                Some(Msg::InfoPopup(InfoPopupMsg::Closed))
            }
            _ => None,
        }
    }
}
