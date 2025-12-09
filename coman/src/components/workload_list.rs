use std::cmp::Reverse;

use tui_realm_stdlib::Table;
use tuirealm::{
    AttrValue, Attribute, Component, Event, MockComponent, State, StateValue,
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent, KeyModifiers},
    props::{Alignment, BorderType, Borders, Color, TableBuilder, TextSpan},
};

use crate::{
    app::{
        messages::{JobMsg, Msg},
        user_events::{CscsEvent, UserEvent},
    },
    cscs::api_client::Job,
};

#[derive(MockComponent)]
pub(crate) struct WorkloadList {
    component: Table,
    jobs: Vec<Job>,
}

impl Default for WorkloadList {
    fn default() -> Self {
        Self {
            component: Table::default()
                .borders(Borders::default().modifiers(BorderType::Rounded).color(Color::Yellow))
                .title("Workloads", Alignment::Center)
                .scroll(true)
                .highlighted_color(Color::LightYellow)
                .highlighted_str("❯ ")
                .rewind(true)
                .step(4)
                .headers(["Name", "Status", "Id", "Start", "End"]),
            jobs: vec![],
        }
    }
}

impl Component<Msg, UserEvent> for WorkloadList {
    fn on(&mut self, ev: tuirealm::Event<UserEvent>) -> Option<Msg> {
        let _ = match ev {
            Event::Keyboard(KeyEvent { code: Key::Down, .. }) => self.perform(Cmd::Move(Direction::Down)),
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => self.perform(Cmd::Move(Direction::Up)),
            Event::Keyboard(KeyEvent {
                code: Key::PageDown, ..
            }) => self.perform(Cmd::Scroll(Direction::Down)),
            Event::Keyboard(KeyEvent { code: Key::PageUp, .. }) => self.perform(Cmd::Scroll(Direction::Up)),
            Event::Keyboard(KeyEvent { code: Key::Home, .. }) => self.perform(Cmd::GoTo(Position::Begin)),
            Event::Keyboard(KeyEvent { code: Key::End, .. }) => self.perform(Cmd::GoTo(Position::End)),
            Event::User(UserEvent::Cscs(CscsEvent::GotWorkloadData(data))) => {
                if data.is_empty() {
                    self.jobs = vec![];
                    self.attr(Attribute::Content, AttrValue::Table(vec![]));
                } else {
                    self.jobs = data.clone();
                    self.jobs.sort_by_key(|j| Reverse(j.start_date));
                    let mut table = TableBuilder::default();
                    for (idx, job) in self.jobs.iter().enumerate() {
                        if idx > 0 {
                            table.add_row();
                        }
                        table
                            .add_col(TextSpan::from(job.name.clone()).bold())
                            .add_col(TextSpan::from(job.status.to_string()))
                            .add_col(TextSpan::from(job.id.to_string()))
                            .add_col(TextSpan::from(
                                job.start_date
                                    .map(|s| s.format("%Y-%m-%d %H:%M").to_string())
                                    .unwrap_or("".to_owned()),
                            ))
                            .add_col(TextSpan::from(
                                job.end_date
                                    .map(|s| s.format("%Y-%m-%d %H:%M").to_string())
                                    .unwrap_or("".to_owned()),
                            ));
                    }
                    self.attr(Attribute::Content, AttrValue::Table(table.build()));
                }
                self.perform(Cmd::Change)
            }
            Event::Keyboard(KeyEvent { code: Key::Enter, .. }) => {
                if let State::One(StateValue::Usize(index)) = self.state()
                    && !self.jobs.is_empty()
                {
                    let job = self.jobs[index].clone();
                    return Some(Msg::Job(JobMsg::GetDetails(job.id)));
                }
                CmdResult::None
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('l'),
                modifiers: KeyModifiers::NONE,
            }) => {
                if let State::One(StateValue::Usize(index)) = self.state()
                    && !self.jobs.is_empty()
                {
                    let job = self.jobs[index].clone();
                    return Some(Msg::Job(JobMsg::Log(job.id)));
                }
                CmdResult::None
            }
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
