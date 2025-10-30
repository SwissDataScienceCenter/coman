use tui_realm_stdlib::Paragraph;
use tuirealm::{
    Component, Event,
    event::{Key, KeyEvent},
    props::{Alignment, BorderType, Borders, Color},
};

use crate::app::{
    messages::{ErrorPopupMsg, Msg},
    user_events::UserEvent,
};

#[derive(MockComponent)]
pub struct ErrorPopup {
    component: Paragraph,
}

impl Default for ErrorPopup {
    fn default() -> Self {
        Self {
            component: Paragraph::default()
                .borders(
                    Borders::default()
                        .modifiers(BorderType::Thick)
                        .color(Color::Red),
                )
                .title("Error", Alignment::Left),
        }
    }
}

impl Component<Msg, UserEvent> for ErrorPopup {
    fn on(&mut self, ev: tuirealm::Event<UserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent { code: Key::Esc, .. })
            | Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
            }) => Some(Msg::ErrorPopup(ErrorPopupMsg::Closed)),
            _ => None,
        }
    }
}
