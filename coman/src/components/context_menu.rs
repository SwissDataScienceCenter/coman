use tui_realm_stdlib::components::List;
use tuirealm::{
    command::{Cmd, CmdResult, Direction, Position},
    component::{AppComponent, Component},
    event::{Event, Key, KeyEvent},
    props::{AttrValue, Attribute, BorderType, Borders, Color, PropPayload, PropValue},
    ratatui::{style::Style, text::Line},
    state::{State, StateValue},
};

use crate::app::{
    messages::{MenuMsg, Msg, View},
    user_events::{FileEvent, JobEvent, UserEvent},
};

#[derive(Component)]
pub struct ContextMenu {
    component: List,
    current_view: View,
}

impl ContextMenu {
    fn workload_options() -> Vec<Line<'static>> {
        vec![
            Line::styled("Cancel Job", Style::new().fg(Color::Cyan)),
            Line::styled("Filter by Status", Style::new().fg(Color::Cyan)),
            Line::styled("Login to CSCS", Style::new().fg(Color::Cyan)),
            Line::styled("Switch System", Style::new().fg(Color::Cyan)),
            Line::styled("Quit", Style::new().fg(Color::Cyan)),
        ]
    }
    fn workload_actions(index: usize) -> Option<Msg> {
        match index {
            0 => Some(Msg::Menu(MenuMsg::Event(UserEvent::Job(JobEvent::Cancel)))),
            1 => Some(Msg::Menu(MenuMsg::CscsShowFilterPopup)),
            2 => Some(Msg::Menu(MenuMsg::CscsSwitchSystem)),
            3 => Some(Msg::AppClose),
            _ => Some(Msg::Menu(MenuMsg::Closed)),
        }
    }
    fn fileview_options() -> Vec<Line<'static>> {
        vec![
            Line::styled("Login to CSCS", Style::new().fg(Color::Cyan)),
            Line::styled("Switch System", Style::new().fg(Color::Cyan)),
            Line::styled("Download", Style::new().fg(Color::Cyan)),
            Line::styled("Delete", Style::new().fg(Color::Cyan)),
            Line::styled("Quit", Style::new().fg(Color::Cyan)),
        ]
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
                .title("Menu")
                .scroll(true)
                .highlight_style(Style::new().bg(Color::LightYellow))
                .highlight_str("-")
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

impl AppComponent<Msg, UserEvent> for ContextMenu {
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
                return Some(Msg::Menu(MenuMsg::Closed));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('x'), ..
            }) => {
                return Some(Msg::Menu(MenuMsg::Closed));
            }
            Event::Keyboard(KeyEvent { code: Key::Enter, .. }) => {
                let msg = if let State::Single(StateValue::Usize(index)) = self.state() {
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
                    View::Workloads => self.attr(
                        Attribute::Text,
                        AttrValue::Payload(PropPayload::Vec(
                            ContextMenu::workload_options()
                                .into_iter().map(PropValue::TextLine)
                                .collect(),
                        )),
                    ),
                    View::Files => self.attr(
                        Attribute::Text,
                        AttrValue::Payload(PropPayload::Vec(
                            ContextMenu::fileview_options()
                                .into_iter().map(PropValue::TextLine)
                                .collect(),
                        )),
                    ),
                };
                self.current_view = view.to_owned();
                CmdResult::NoChange
            }

            _ => CmdResult::NoChange,
        };
        Some(Msg::None)
    }
}
