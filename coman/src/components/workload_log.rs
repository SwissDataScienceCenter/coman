use tui_realm_stdlib::Textarea;
use tuirealm::{
    AttrValue, Attribute, Component, Event, MockComponent,
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent},
    props::{Alignment, BorderType, Borders, Color, PropPayload, PropValue, TextSpan},
};

use crate::{
    app::{
        messages::{JobMsg, Msg},
        user_events::{CscsEvent, UserEvent},
    },
    trace_dbg,
};

#[derive(MockComponent)]
pub struct WorkloadLog {
    component: Textarea,
}

impl WorkloadLog {
    pub fn new() -> Self {
        Self {
            component: Textarea::default()
                .borders(Borders::default().modifiers(BorderType::Rounded).color(Color::Yellow))
                .title("Workload Log", Alignment::Center)
                .step(4),
        }
    }
}
impl Component<Msg, UserEvent> for WorkloadLog {
    fn on(&mut self, ev: tuirealm::Event<UserEvent>) -> Option<Msg> {
        let _ = match ev {
            Event::User(UserEvent::Cscs(CscsEvent::GotJobLog(log))) => {
                let _ = trace_dbg!("got log component");
                let log = trace_dbg!(log);
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
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => {
                return Some(Msg::Job(JobMsg::CloseLog));
            }
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
