use tui_realm_stdlib::Textarea;
use tuirealm::{
    AttrValue, Attribute, Component, Event, MockComponent,
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent},
    props::{Alignment, BorderType, Borders, Color, PropPayload, PropValue, TextSpan},
};

use crate::app::{
    messages::{JobMsg, Msg},
    user_events::{CscsEvent, UserEvent},
};

#[derive(MockComponent)]
pub struct WorkloadLog {
    component: Textarea,
    stderr: bool,
}

impl WorkloadLog {
    pub fn new() -> Self {
        Self {
            component: Textarea::default()
                .borders(Borders::default().modifiers(BorderType::Rounded).color(Color::Yellow))
                .title("Workload Log (stdout)", Alignment::Center)
                .step(4),
            stderr: false,
        }
    }
}
impl Component<Msg, UserEvent> for WorkloadLog {
    fn on(&mut self, ev: tuirealm::Event<UserEvent>) -> Option<Msg> {
        let _ = match ev {
            Event::User(UserEvent::Cscs(CscsEvent::GotJobLog(log))) => {
                self.attr(
                    Attribute::Text,
                    AttrValue::Payload(PropPayload::Vec(
                        log.lines().map(|l| PropValue::TextSpan(TextSpan::from(l))).collect(),
                    )),
                );
                self.perform(Cmd::Change)
            }
            Event::Keyboard(KeyEvent { code: Key::Down, .. }) => self.perform(Cmd::Move(Direction::Down)),
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => self.perform(Cmd::Move(Direction::Up)),
            Event::Keyboard(KeyEvent {
                code: Key::PageDown, ..
            }) => self.perform(Cmd::Scroll(Direction::Down)),
            Event::Keyboard(KeyEvent { code: Key::PageUp, .. }) => self.perform(Cmd::Scroll(Direction::Up)),
            Event::Keyboard(KeyEvent { code: Key::Home, .. }) => self.perform(Cmd::GoTo(Position::Begin)),
            Event::Keyboard(KeyEvent { code: Key::End, .. }) => self.perform(Cmd::GoTo(Position::End)),
            Event::Keyboard(KeyEvent { code: Key::Tab, .. }) => {
                self.stderr = !self.stderr;
                if self.stderr {
                    self.attr(
                        Attribute::Title,
                        AttrValue::Title(("Workload Log (stderr)".to_owned(), Alignment::Center)),
                    );
                } else {
                    self.attr(
                        Attribute::Title,
                        AttrValue::Title(("Workload Log (stdout)".to_owned(), Alignment::Center)),
                    );
                }
                // empty log view
                self.attr(Attribute::Text, AttrValue::Payload(PropPayload::Vec(vec![])));
                return Some(Msg::Job(JobMsg::Switch));
            }
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => {
                return Some(Msg::Job(JobMsg::Close));
            }
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
