use tui_realm_stdlib::components::Textarea;
use tuirealm::{
    command::{Cmd, CmdResult, Direction, Position},
    component::{AppComponent, Component},
    event::{Event, Key, KeyEvent},
    props::{AttrValue, Attribute, BorderType, Borders, Color, PropPayload, PropValue, Title},
    ratatui::text::Span,
};

use crate::app::{
    messages::{JobMsg, Msg},
    user_events::{CscsEvent, UserEvent},
};

#[derive(Component)]
pub struct WorkloadLog {
    component: Textarea,
    stderr: bool,
}

impl WorkloadLog {
    pub fn new() -> Self {
        Self {
            component: Textarea::default()
                .borders(Borders::default().modifiers(BorderType::Rounded).color(Color::Yellow))
                .title("Workload Log (stdout)")
                .step(4),
            stderr: false,
        }
    }
}
impl AppComponent<Msg, UserEvent> for WorkloadLog {
    fn on(&mut self, ev: &Event<UserEvent>) -> Option<Msg> {
        let _ = match ev {
            Event::User(UserEvent::Cscs(CscsEvent::GotJobLog(log))) => {
                let l = log.clone();
                let spans: Vec<PropValue> = l
                    .lines()
                    .map(|l| PropValue::TextSpan(Span::from(l.to_owned())))
                    .collect();
                self.attr(Attribute::Text, AttrValue::Payload(PropPayload::Vec(spans.clone())));
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
                        AttrValue::Title(Title::from("Workload Log (stderr)".to_owned())),
                    );
                } else {
                    self.attr(
                        Attribute::Title,
                        AttrValue::Title(Title::from("Workload Log (stdout)".to_owned())),
                    );
                }
                // empty log view
                self.attr(Attribute::Text, AttrValue::Payload(PropPayload::Vec(vec![])));
                return Some(Msg::Job(JobMsg::Switch));
            }
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => {
                return Some(Msg::Job(JobMsg::Close));
            }
            _ => CmdResult::NoChange,
        };
        Some(Msg::None)
    }
}
