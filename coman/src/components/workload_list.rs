use std::cmp::Reverse;

use tui_realm_stdlib::components::Table;
use tuirealm::{
    command::{Cmd, CmdResult, Direction, Position},
    component::{AppComponent, Component},
    event::{Event, Key, KeyEvent, KeyModifiers},
    props::{AttrValue, Attribute, BorderType, Borders, Color, TableBuilder},
    ratatui::{style::Style, text::Line},
    state::{State, StateValue},
};

use crate::{
    app::{
        messages::{JobMsg, Msg},
        user_events::{CscsEvent, JobEvent, UserEvent},
    },
    cscs::api_client::types::{Job, JobStatus},
};

#[derive(Component)]
pub(crate) struct WorkloadList {
    component: Table,
    jobs: Vec<Job>,
}

impl Default for WorkloadList {
    fn default() -> Self {
        Self {
            component: Table::default()
                .borders(Borders::default().modifiers(BorderType::Rounded).color(Color::Yellow))
                .title("Workloads")
                .scroll(true)
                .highlight_style(Style::new().bg(Color::LightYellow))
                .highlight_str("❯ ")
                .rewind(true)
                .step(4)
                .headers(["Name", "Status", "Id", "Start", "End"]),
            jobs: vec![],
        }
    }
}

impl AppComponent<Msg, UserEvent> for WorkloadList {
    fn on(&mut self, ev: &Event<UserEvent>) -> Option<Msg> {
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
                            .add_col(Line::styled(job.name.clone(), Style::new().bold()))
                            .add_col(Line::from(job.status.to_string()))
                            .add_col(Line::from(job.id.to_string()))
                            .add_col(Line::from(
                                job.start_date
                                    .map(|s| s.format("%Y-%m-%d %H:%M").to_string())
                                    .unwrap_or("".to_owned()),
                            ))
                            .add_col(Line::from(
                                job.end_date
                                    .map(|s| s.format("%Y-%m-%d %H:%M").to_string())
                                    .unwrap_or("".to_owned()),
                            ));
                    }
                    self.attr(Attribute::Content, AttrValue::Table(table.build()));
                }
                self.perform(Cmd::Change)
            }
            Event::User(UserEvent::Job(JobEvent::Cancel)) => {
                if let State::Single(StateValue::Usize(index)) = self.state()
                    && !self.jobs.is_empty()
                {
                    let job = self.jobs[index].clone();
                    return Some(Msg::Job(JobMsg::Cancel(job.id)));
                }
                CmdResult::NoChange
            }
            Event::Keyboard(KeyEvent { code: Key::Enter, .. }) => {
                if let State::Single(StateValue::Usize(index)) = self.state()
                    && !self.jobs.is_empty()
                {
                    let job = self.jobs[index].clone();
                    return Some(Msg::Job(JobMsg::GetDetails(job.id)));
                }
                CmdResult::NoChange
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('l'),
                modifiers: KeyModifiers::NONE,
            }) => {
                if let State::Single(StateValue::Usize(index)) = self.state()
                    && !self.jobs.is_empty()
                {
                    let job = self.jobs[index].clone();
                    return Some(Msg::Job(JobMsg::Log(job.id)));
                }
                CmdResult::NoChange
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('r'),
                modifiers: KeyModifiers::NONE,
            }) => {
                if let State::Single(StateValue::Usize(index)) = self.state()
                    && !self.jobs.is_empty()
                {
                    let job = self.jobs[index].clone();
                    if job.status != JobStatus::Running {
                        return Some(Msg::Error(
                            "Can only get resource usage for jobs in 'Running' state".to_string(),
                        ));
                    }
                    return Some(Msg::Job(JobMsg::ResourceUsage(job.id)));
                }
                CmdResult::NoChange
            }
            _ => CmdResult::NoChange,
        };
        Some(Msg::None)
    }
}
