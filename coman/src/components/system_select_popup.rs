use tui_realm_stdlib::List;
use tuirealm::{
    AttrValue, Attribute, Component, Event, Frame, MockComponent, State, StateValue,
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    props::{Alignment, BorderType, Borders, Color, TableBuilder, TextSpan},
    ratatui::layout::{Position as RectPosition, Rect},
};

use crate::{
    app::{
        messages::{Msg, SystemSelectMsg},
        user_events::UserEvent,
    },
    cscs::api_client::types::System,
};

pub struct SystemSelectPopup {
    component: List,
    systems: Vec<System>,
    current_rect: Rect,
}

impl MockComponent for SystemSelectPopup {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.current_rect = area;
        self.component.view(frame, area);
    }
    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.component.query(attr)
    }
    fn attr(&mut self, query: Attribute, attr: AttrValue) {
        self.component.attr(query, attr)
    }
    fn state(&self) -> State {
        self.component.state()
    }
    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        self.component.perform(cmd)
    }
}

impl SystemSelectPopup {
    pub fn new(systems: Vec<System>) -> Self {
        let mut rows = TableBuilder::default();
        for system in systems.clone() {
            rows.add_col(TextSpan::from(system.name).fg(Color::Cyan)).add_row();
        }
        Self {
            component: List::default()
                .borders(Borders::default().modifiers(BorderType::Thick).color(Color::Green))
                .title("Select System", Alignment::Left)
                .scroll(true)
                .highlighted_color(Color::LightYellow)
                .highlighted_str("-")
                .rewind(true)
                .step(4)
                .rows(rows.build()),
            systems,
            current_rect: Rect::ZERO,
        }
    }

    fn select_entry(&mut self) -> Option<Msg> {
        if let State::One(StateValue::Usize(index)) = self.state() {
            let selected_system = self.systems[index].clone();
            Some(Msg::SystemSelectPopup(SystemSelectMsg::SystemSelected(
                selected_system.name,
            )))
        } else {
            Some(Msg::SystemSelectPopup(SystemSelectMsg::Closed))
        }
    }
}

impl Component<Msg, UserEvent> for SystemSelectPopup {
    fn on(&mut self, ev: Event<UserEvent>) -> Option<Msg> {
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
                return self.select_entry();
            }
            Event::Mouse(MouseEvent {
                kind, column: col, row, ..
            }) => {
                if !self.current_rect.contains(RectPosition { x: col, y: row }) {
                    CmdResult::None
                } else {
                    let mut list_index = (row - self.current_rect.y) as usize;
                    list_index = list_index.saturating_sub(1);
                    if list_index >= self.component.states.list_len {
                        list_index = self.component.states.list_len;
                    }

                    match kind {
                        MouseEventKind::Moved => {
                            self.component.states.list_index = list_index;
                            CmdResult::Changed(self.component.state())
                        }
                        MouseEventKind::Down(MouseButton::Left) => {
                            return self.select_entry();
                        }
                        MouseEventKind::ScrollUp => self.perform(Cmd::Move(Direction::Up)),
                        MouseEventKind::ScrollDown => self.perform(Cmd::Move(Direction::Down)),
                        _ => CmdResult::None,
                    }
                }
            }

            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
