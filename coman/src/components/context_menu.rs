use tui_realm_stdlib::List;
use tuirealm::{
    AttrValue, Attribute, Component, Event, MockComponent, State, StateValue,
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent},
    props::{Alignment, BorderType, Borders, Color, Table, TableBuilder, TextSpan},
};

use crate::app::{
    messages::{MenuMsg, Msg, View},
    user_events::{FileEvent, UserEvent},
};

#[derive(MockComponent)]
pub struct ContextMenu {
    component: List,
    current_view: View,
}

impl ContextMenu {
    fn workload_options() -> Table {
        TableBuilder::default()
            .add_col(TextSpan::from("Login to CSCS").fg(Color::Cyan))
            .add_row()
            .add_col(TextSpan::from("Switch System").fg(Color::Cyan))
            .add_row()
            .add_col(TextSpan::from("Quit").fg(Color::Cyan))
            .add_row()
            .build()
    }
    fn workload_actions(index: usize) -> Option<Msg> {
        match index {
            0 => Some(Msg::Menu(MenuMsg::CscsLogin)),
            1 => Some(Msg::Menu(MenuMsg::CscsSwitchSystem)),
            2 => Some(Msg::AppClose),
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
            .add_col(TextSpan::from("Quit").fg(Color::Cyan))
            .add_row()
            .build()
    }
    fn fileview_actions(index: usize) -> Option<Msg> {
        match index {
            0 => Some(Msg::Menu(MenuMsg::CscsLogin)),
            1 => Some(Msg::Menu(MenuMsg::CscsSwitchSystem)),
            2 => Some(Msg::Menu(MenuMsg::Event(UserEvent::File(
                FileEvent::DownloadCurrentFile,
            )))),
            3 => Some(Msg::AppClose),
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
        }
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
