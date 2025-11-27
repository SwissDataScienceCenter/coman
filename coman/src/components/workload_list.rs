use tui_realm_stdlib::List;
use tuirealm::{
    AttrValue, Attribute, Component, Event, MockComponent,
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent},
    props::{Alignment, BorderType, Borders, Color, TableBuilder, TextSpan},
};

use crate::app::{
    messages::Msg,
    user_events::{CscsEvent, UserEvent},
};

#[derive(MockComponent)]
pub(crate) struct WorkloadList {
    component: List,
}

impl Default for WorkloadList {
    fn default() -> Self {
        Self {
            component: List::default()
                .borders(
                    Borders::default()
                        .modifiers(BorderType::Rounded)
                        .color(Color::Yellow),
                )
                .title("Workloads", Alignment::Center)
                .scroll(true)
                .highlighted_color(Color::LightYellow)
                .highlighted_str("-")
                .rewind(true)
                .step(4),
        }
    }
}

impl Component<Msg, UserEvent> for WorkloadList {
    fn on(&mut self, ev: tuirealm::Event<UserEvent>) -> Option<Msg> {
        let _ = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => self.perform(Cmd::Move(Direction::Down)),
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => {
                self.perform(Cmd::Move(Direction::Up))
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                ..
            }) => self.perform(Cmd::Scroll(Direction::Down)),
            Event::Keyboard(KeyEvent {
                code: Key::PageUp, ..
            }) => self.perform(Cmd::Scroll(Direction::Up)),
            Event::Keyboard(KeyEvent {
                code: Key::Home, ..
            }) => self.perform(Cmd::GoTo(Position::Begin)),
            Event::Keyboard(KeyEvent { code: Key::End, .. }) => {
                self.perform(Cmd::GoTo(Position::End))
            }
            Event::User(UserEvent::Cscs(CscsEvent::GotWorkloadData(data))) => {
                if data.len() == 0 {
                    self.attr(Attribute::Content, AttrValue::Table(vec![]));
                } else {
                    let mut table = TableBuilder::default();
                    for (idx, job) in data.iter().enumerate() {
                        if idx > 0 {
                            table.add_row();
                        }
                        table
                            .add_col(TextSpan::from(job.name.clone()).bold())
                            .add_col(TextSpan::from(" "))
                            .add_col(TextSpan::from(job.status.to_string()))
                            .add_col(TextSpan::from(" "))
                            .add_col(TextSpan::from(job.id.to_string()));
                    }
                    self.attr(Attribute::Content, AttrValue::Table(table.build()));
                }
                self.perform(Cmd::Change)
            }
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
