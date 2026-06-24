use tui_realm_stdlib::components::List;
use tuirealm::{
    command::{Cmd, CmdResult, Direction, Position},
    component::{AppComponent, Component},
    event::{Event, Key, KeyEvent},
    props::{BorderType, Borders, Color},
    ratatui::{style::Style, text::Line},
    state::{State, StateValue},
};

use crate::{
    app::{
        messages::{Msg, SystemSelectMsg},
        user_events::UserEvent,
    },
    cscs::api_client::types::System,
};

#[derive(Component)]
pub struct SystemSelectPopup {
    component: List,
    systems: Vec<System>,
}

impl SystemSelectPopup {
    pub fn new(systems: Vec<System>) -> Self {
        let mut rows = vec![];
        for system in systems.clone() {
            rows.push(Line::styled(system.name, Style::new().fg(Color::Cyan)));
        }
        Self {
            component: List::default()
                .borders(Borders::default().modifiers(BorderType::Thick).color(Color::Green))
                .title("Select System")
                .scroll(true)
                .highlight_style(Style::new().bg(Color::LightYellow))
                .highlight_str("-")
                .rewind(true)
                .step(4)
                .rows(rows),
            systems,
        }
    }
}

impl AppComponent<Msg, UserEvent> for SystemSelectPopup {
    fn on(&mut self, ev: &Event<UserEvent>) -> Option<Msg> {
        let _ = match ev {
            Event::Keyboard(KeyEvent { code: Key::Down, .. }) => self.perform(Cmd::Move(Direction::Down)),
            Event::Keyboard(KeyEvent {
                code: Key::PageDown, ..
            }) => self.perform(Cmd::Move(Direction::Down)),
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => self.perform(Cmd::Move(Direction::Up)),
            Event::Keyboard(KeyEvent { code: Key::PageUp, .. }) => self.perform(Cmd::Move(Direction::Up)),
            Event::Keyboard(KeyEvent { code: Key::Home, .. }) => self.perform(Cmd::GoTo(Position::Begin)),
            Event::Keyboard(KeyEvent { code: Key::End, .. }) => self.perform(Cmd::GoTo(Position::End)),
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => {
                return Some(Msg::SystemSelectPopup(SystemSelectMsg::Closed));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('x'), ..
            }) => {
                return Some(Msg::SystemSelectPopup(SystemSelectMsg::Closed));
            }
            Event::Keyboard(KeyEvent { code: Key::Enter, .. }) => {
                let msg = if let State::Single(StateValue::Usize(index)) = self.state() {
                    let selected_system = self.systems[index].clone();
                    Some(Msg::SystemSelectPopup(SystemSelectMsg::SystemSelected(
                        selected_system.name,
                    )))
                } else {
                    Some(Msg::SystemSelectPopup(SystemSelectMsg::Closed))
                };
                return msg;
            }

            _ => CmdResult::NoChange,
        };
        Some(Msg::None)
    }
}
