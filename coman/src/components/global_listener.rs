use tui_realm_stdlib::Phantom;
use tuirealm::{
    Component, Event, MockComponent, Props, State,
    command::CmdResult,
    event::{Key, KeyEvent, KeyModifiers},
};

use crate::app::{
    messages::{MenuMsg, Msg},
    user_events::UserEvent,
};

#[derive(Default, MockComponent)]
pub struct GlobalListener {
    component: Phantom,
}

impl Component<Msg, UserEvent> for GlobalListener {
    fn on(&mut self, ev: tuirealm::Event<UserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                ..
            })
            | Event::Keyboard(KeyEvent {
                code: Key::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            }) => Some(Msg::AppClose),
            Event::Keyboard(KeyEvent {
                code: Key::Char('x'),
                ..
            }) => Some(Msg::Menu(MenuMsg::Opened)),
            _ => None,
        }
    }
}
