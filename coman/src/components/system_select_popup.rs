use tui_realm_stdlib::List;
use tuirealm::{
    Component, Event, Frame, MockComponent, State, StateValue,
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent, KeyModifiers},
    props::{
        Alignment, AttrValue, Attribute, BorderType, Borders, Color, InputType, Layout, Props,
        Style, TableBuilder, TextSpan,
    },
    ratatui::{
        layout::{Constraint, Direction as LayoutDirection, Rect},
        widgets::Block,
    },
};

use crate::{
    app::{
        messages::{Msg, SystemSelectMsg},
        user_events::UserEvent,
    },
    cscs::api_client::System,
};

#[derive(MockComponent)]
pub struct SystemSelectPopup {
    component: List,
    systems: Vec<System>,
}

impl SystemSelectPopup {
    pub fn new(systems: Vec<System>) -> Self {
        let mut rows = TableBuilder::default();
        for system in systems.clone() {
            rows.add_col(TextSpan::from(system.name).fg(Color::Cyan))
                .add_row();
        }
        Self {
            component: List::default()
                .borders(
                    Borders::default()
                        .modifiers(BorderType::Thick)
                        .color(Color::Green),
                )
                .title("Select System", Alignment::Left)
                .scroll(true)
                .highlighted_color(Color::LightYellow)
                .highlighted_str("-")
                .rewind(true)
                .step(4)
                .rows(rows.build()),
            systems,
        }
    }
}

impl Component<Msg, UserEvent> for SystemSelectPopup {
    fn on(&mut self, ev: Event<UserEvent>) -> Option<Msg> {
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
                return Some(Msg::SystemSelectPopup(SystemSelectMsg::Closed));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('x'),
                ..
            }) => {
                return Some(Msg::SystemSelectPopup(SystemSelectMsg::Closed));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
            }) => {
                let msg = if let State::One(StateValue::Usize(index)) = self.state() {
                    let selected_system = self.systems[index].clone();
                    Some(Msg::SystemSelectPopup(SystemSelectMsg::SystemSelected(
                        selected_system.name,
                    )))
                } else {
                    Some(Msg::SystemSelectPopup(SystemSelectMsg::Closed))
                };
                return msg;
            }

            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
