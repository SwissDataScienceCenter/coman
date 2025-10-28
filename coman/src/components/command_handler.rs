use tuirealm::{
    Component, Event, MockComponent, Props, State,
    command::CmdResult,
    event::{Key, KeyEvent},
};

use crate::app::{messages::Msg, user_events::UserEvent};

#[derive(Default)]
pub struct CommandHandler {
    props: Props,
}

impl MockComponent for CommandHandler {
    fn view(&mut self, _frame: &mut ratatui::Frame, _area: ratatui::prelude::Rect) {}

    fn query(&self, attr: tuirealm::Attribute) -> Option<tuirealm::AttrValue> {
        self.props.get(attr)
    }

    fn attr(&mut self, attr: tuirealm::Attribute, value: tuirealm::AttrValue) {
        self.props.set(attr, value);
    }

    fn state(&self) -> tuirealm::State {
        State::None
    }

    fn perform(&mut self, _cmd: tuirealm::command::Cmd) -> tuirealm::command::CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, UserEvent> for CommandHandler {
    fn on(&mut self, ev: tuirealm::Event<UserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                ..
            }) => Some(Msg::AppClose),
            _ => None,
        }
    }
}
