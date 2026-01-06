use tui_realm_stdlib::List;
use tuirealm::{
    AttrValue, Attribute, Component, Event, Frame, MockComponent, State, StateValue,
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    props::{Alignment, BorderType, Borders, Color, Table, TableBuilder, TextSpan},
    ratatui::layout::{Position as RectPosition, Rect},
};

use crate::app::{
    messages::{MenuMsg, Msg, View},
    user_events::{FileEvent, JobEvent, UserEvent},
};

pub struct ContextMenu {
    component: List,
    current_view: View,
    current_rect: Rect,
}

impl ContextMenu {
    fn workload_options() -> Table {
        TableBuilder::default()
            .add_col(TextSpan::from("Login to CSCS").fg(Color::Cyan))
            .add_row()
            .add_col(TextSpan::from("Switch System").fg(Color::Cyan))
            .add_row()
            .add_col(TextSpan::from("Cancel Job").fg(Color::Cyan))
            .add_row()
            .add_col(TextSpan::from("Quit").fg(Color::Cyan))
            .build()
    }
    fn workload_actions(index: usize) -> Option<Msg> {
        match index {
            0 => Some(Msg::Menu(MenuMsg::CscsLogin)),
            1 => Some(Msg::Menu(MenuMsg::CscsSwitchSystem)),
            2 => Some(Msg::Menu(MenuMsg::Event(UserEvent::Job(JobEvent::Cancel)))),
            3 => Some(Msg::AppClose),
            _ => Some(Msg::Menu(MenuMsg::Closed)),
        }
    }
    fn fileview_options() -> Table {
        TableBuilder::default()
            .add_col(TextSpan::from("Login to CSCS").fg(Color::Cyan))
            .add_row()
            .add_col(TextSpan::from("Switch System").fg(Color::Cyan))
            .add_row()
            .add_col(TextSpan::from("Download").fg(Color::Cyan))
            .add_row()
            .add_col(TextSpan::from("Delete").fg(Color::Cyan))
            .add_row()
            .add_col(TextSpan::from("Quit").fg(Color::Cyan))
            .build()
    }
    fn fileview_actions(index: usize) -> Option<Msg> {
        match index {
            0 => Some(Msg::Menu(MenuMsg::CscsLogin)),
            1 => Some(Msg::Menu(MenuMsg::CscsSwitchSystem)),
            2 => Some(Msg::Menu(MenuMsg::Event(UserEvent::File(
                FileEvent::DownloadCurrentFile,
            )))),
            3 => Some(Msg::Menu(MenuMsg::Event(UserEvent::File(FileEvent::DeleteCurrentFile)))),
            4 => Some(Msg::AppClose),
            _ => Some(Msg::Menu(MenuMsg::Closed)),
        }
    }

    pub fn new(view: View) -> Self {
        Self {
            component: List::default()
                .borders(Borders::default().modifiers(BorderType::Thick).color(Color::Yellow))
                .title("Menu", Alignment::Left)
                .scroll(true)
                .highlighted_color(Color::LightYellow)
                .highlighted_str("-")
                .rewind(true)
                .step(4)
                .rows(match view {
                    View::Workloads => ContextMenu::workload_options(),
                    View::Files => ContextMenu::fileview_options(),
                })
                .selected_line(0),
            current_view: view,
            current_rect: Rect::ZERO,
        }
    }
}

impl MockComponent for ContextMenu {
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

impl Component<Msg, UserEvent> for ContextMenu {
    fn on(&mut self, ev: tuirealm::Event<UserEvent>) -> Option<Msg> {
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
                return Some(Msg::Menu(MenuMsg::Closed));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('x'), ..
            }) => {
                return Some(Msg::Menu(MenuMsg::Closed));
            }
            Event::Keyboard(KeyEvent { code: Key::Enter, .. }) => {
                let msg = if let State::One(StateValue::Usize(index)) = self.state() {
                    match self.current_view {
                        View::Workloads => ContextMenu::workload_actions(index),
                        View::Files => ContextMenu::fileview_actions(index),
                    }
                } else {
                    Some(Msg::Menu(MenuMsg::Closed))
                };
                return msg;
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
                            return match self.current_view {
                                View::Workloads => ContextMenu::workload_actions(list_index),
                                View::Files => ContextMenu::fileview_actions(list_index),
                            };
                        }
                        MouseEventKind::ScrollUp => self.perform(Cmd::Move(Direction::Up)),
                        MouseEventKind::ScrollDown => self.perform(Cmd::Move(Direction::Down)),
                        _ => CmdResult::None,
                    }
                }
            }
            Event::User(UserEvent::SwitchedToView(view)) => {
                match view {
                    View::Workloads => self.attr(Attribute::Content, AttrValue::Table(ContextMenu::workload_options())),
                    View::Files => self.attr(Attribute::Content, AttrValue::Table(ContextMenu::fileview_options())),
                };
                self.current_view = view;
                CmdResult::None
            }

            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
