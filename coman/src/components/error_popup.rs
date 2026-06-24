use tui_realm_stdlib::components::Paragraph;
use tuirealm::{
    component::{AppComponent, Component},
    event::{Event, Key, KeyEvent},
    props::{BorderType, Borders, Color, TextStatic},
};

use crate::app::{
    messages::{ErrorPopupMsg, Msg},
    user_events::UserEvent,
};

#[derive(Component)]
pub struct ErrorPopup {
    component: Paragraph,
}

impl ErrorPopup {
    pub fn new<S: Into<String>>(msg: S) -> Self {
        Self {
            component: Paragraph::default()
                .borders(Borders::default().modifiers(BorderType::Thick).color(Color::Red))
                .title("Error")
                .text(TextStatic::from(msg.into())),
        }
    }
}

impl AppComponent<Msg, UserEvent> for ErrorPopup {
    fn on(&mut self, ev: &Event<UserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) | Event::Keyboard(KeyEvent { code: Key::Enter, .. }) => {
                Some(Msg::ErrorPopup(ErrorPopupMsg::Closed))
            }
            _ => None,
        }
    }
}
