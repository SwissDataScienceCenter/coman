use std::path::PathBuf;

use tui_realm_stdlib::components::Input;
use tuirealm::{
    command::{Cmd, CmdResult, Direction, Position},
    component::{AppComponent, Component},
    event::{Event, Key, KeyEvent, KeyModifiers},
    props::{BorderType, Borders, Color, InputType, Style},
    state::{State, StateValue},
};

use crate::app::{
    messages::{DownloadPopupMsg, Msg},
    user_events::UserEvent,
};

#[derive(Component)]
pub struct DownloadTargetInput {
    component: Input,
    remote_path: PathBuf,
}

impl DownloadTargetInput {
    pub fn new(remote_path: PathBuf) -> Self {
        Self {
            component: Input::default()
                .borders(Borders::default().modifiers(BorderType::Thick).color(Color::Green))
                .input_type(InputType::Custom(
                    |path| {
                        let path = PathBuf::from(path);
                        let parent = path.parent();
                        match parent {
                            Some(parent) => parent.exists() && path.to_str().is_some(),
                            None => false,
                        }
                    },
                    |_, _| true,
                ))
                .title("Download Target Path")
                .invalid_style(Style::default().fg(Color::Red)),
            remote_path,
        }
    }
}

impl AppComponent<Msg, UserEvent> for DownloadTargetInput {
    fn on(&mut self, ev: &Event<UserEvent>) -> Option<Msg> {
        let _ = match ev {
            Event::Keyboard(KeyEvent { code: Key::Left, .. }) => self.perform(Cmd::Move(Direction::Left)),
            Event::Keyboard(KeyEvent { code: Key::Right, .. }) => self.perform(Cmd::Move(Direction::Right)),
            Event::Keyboard(KeyEvent { code: Key::Home, .. }) => self.perform(Cmd::GoTo(Position::Begin)),
            Event::Keyboard(KeyEvent { code: Key::End, .. }) => self.perform(Cmd::GoTo(Position::End)),
            Event::Keyboard(KeyEvent { code: Key::Delete, .. }) => self.perform(Cmd::Cancel),
            Event::Keyboard(KeyEvent {
                code: Key::Backspace, ..
            }) => self.perform(Cmd::Delete),
            Event::Keyboard(KeyEvent {
                code: Key::Char(ch),
                modifiers: KeyModifiers::NONE,
            }) => self.perform(Cmd::Type(ch.to_owned())),
            Event::Keyboard(KeyEvent {
                code: Key::Enter,
                modifiers: KeyModifiers::NONE,
            }) => {
                if let State::Single(StateValue::String(target)) = self.state() {
                    let target = PathBuf::from(target);
                    return Some(Msg::DownloadPopup(DownloadPopupMsg::PathSet(
                        self.remote_path.clone(),
                        target,
                    )));
                }

                CmdResult::NoChange
            }
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => {
                return Some(Msg::DownloadPopup(DownloadPopupMsg::Closed));
            }
            _ => CmdResult::NoChange,
        };
        Some(Msg::None)
    }
}
