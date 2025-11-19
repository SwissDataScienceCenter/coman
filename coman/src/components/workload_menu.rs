use tui_realm_stdlib::List;
use tuirealm::{
    Component, Event, MockComponent, State, StateValue,
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent},
    props::{Alignment, BorderType, Borders, Color, TableBuilder, TextSpan},
};

use crate::app::{
    messages::{MenuMsg, Msg},
    user_events::UserEvent,
};

#[derive(MockComponent)]
pub struct WorkloadMenu {
    component: List,
}

impl Default for WorkloadMenu {
    fn default() -> Self {
        Self {
            component: List::default()
                .borders(
                    Borders::default()
                        .modifiers(BorderType::Thick)
                        .color(Color::Yellow),
                )
                .title("Menu", Alignment::Left)
                .scroll(true)
                .highlighted_color(Color::LightYellow)
                .highlighted_str("-")
                .rewind(true)
                .step(4)
                .rows(
                    TableBuilder::default()
                        .add_col(TextSpan::from("Login to CSCS").fg(Color::Cyan))
                        .add_row()
                        .add_col(TextSpan::from("Quit").fg(Color::Cyan))
                        .add_row()
                        .build(),
                )
                .selected_line(0),
        }
    }
}

impl Component<Msg, UserEvent> for WorkloadMenu {
    fn on(&mut self, ev: tuirealm::Event<UserEvent>) -> Option<Msg> {
        let _ = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => self.perform(Cmd::Move(Direction::Down)),
            Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                ..
            }) => self.perform(Cmd::Move(Direction::Down)),
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => {
                self.perform(Cmd::Move(Direction::Up))
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageUp, ..
            }) => self.perform(Cmd::Move(Direction::Up)),
            Event::Keyboard(KeyEvent {
                code: Key::Home, ..
            }) => self.perform(Cmd::GoTo(Position::Begin)),
            Event::Keyboard(KeyEvent { code: Key::End, .. }) => {
                self.perform(Cmd::GoTo(Position::End))
            }
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => {
                return Some(Msg::Menu(MenuMsg::Closed));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('x'),
                ..
            }) => {
                return Some(Msg::Menu(MenuMsg::Closed));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
            }) => {
                let msg = if let State::One(StateValue::Usize(index)) = self.state() {
                    match index {
                        0 => Some(Msg::Menu(MenuMsg::CscsLogin)),
                        _ => Some(Msg::Menu(MenuMsg::Closed)),
                    }
                } else {
                    Some(Msg::Menu(MenuMsg::Closed))
                };
                return msg;
            }

            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
