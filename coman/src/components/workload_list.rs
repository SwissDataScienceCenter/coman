use tui_realm_stdlib::List;
use tuirealm::{
    Component, Event, MockComponent,
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent},
    props::{Alignment, BorderType, Borders, Color},
};

use crate::app::{messages::Msg, user_events::UserEvent};

#[derive(MockComponent)]
pub(crate) struct WorkloadList {
    component: List,
}

impl Default for WorkloadList {
    fn default() -> Self {
        Self {
            component: List::default()
                .borders(
                    Borders::default()
                        .modifiers(BorderType::Rounded)
                        .color(Color::Yellow),
                )
                .title("Workloads", Alignment::Center)
                .scroll(true)
                .highlighted_color(Color::LightYellow)
                .highlighted_str("-")
                .rewind(true)
                .step(4),
        }
    }
}

impl Component<Msg, UserEvent> for WorkloadList {
    fn on(&mut self, ev: tuirealm::Event<UserEvent>) -> Option<Msg> {
        let _ = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => self.perform(Cmd::Move(Direction::Down)),
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => {
                self.perform(Cmd::Move(Direction::Up))
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                ..
            }) => self.perform(Cmd::Scroll(Direction::Down)),
            Event::Keyboard(KeyEvent {
                code: Key::PageUp, ..
            }) => self.perform(Cmd::Scroll(Direction::Up)),
            Event::Keyboard(KeyEvent {
                code: Key::Home, ..
            }) => self.perform(Cmd::GoTo(Position::Begin)),
            Event::Keyboard(KeyEvent { code: Key::End, .. }) => {
                self.perform(Cmd::GoTo(Position::End))
            }
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
