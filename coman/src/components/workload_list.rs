use std::cmp::Reverse;

use tui_realm_stdlib::Table;
use tuirealm::{
    AttrValue, Attribute, Component, Event, Frame, MockComponent, State, StateValue,
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    props::{Alignment, BorderType, Borders, Color, TableBuilder, TextSpan},
    ratatui::layout::{Position as RectPosition, Rect},
};

use crate::{
    app::{
        messages::{JobMsg, Msg},
        user_events::{CscsEvent, JobEvent, UserEvent},
    },
    cscs::api_client::types::Job,
};

pub(crate) struct WorkloadList {
    component: Table,
    jobs: Vec<Job>,
    current_rect: Rect,
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
            current_rect: Rect::ZERO,
        }
    }
}
impl MockComponent for WorkloadList {
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
            Event::User(UserEvent::Job(JobEvent::Cancel)) => {
                if let State::One(StateValue::Usize(index)) = self.state()
                    && !self.jobs.is_empty()
                {
                    let job = self.jobs[index].clone();
                    return Some(Msg::Job(JobMsg::Cancel(job.id)));
                }
                CmdResult::None
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
                            if !self.jobs.is_empty() {
                                let job = self.jobs[list_index].clone();
                                return Some(Msg::Job(JobMsg::Log(job.id)));
                            }
                            CmdResult::None
                        }
                        MouseEventKind::Down(MouseButton::Right) => {
                            if !self.jobs.is_empty() {
                                let job = self.jobs[list_index].clone();
                                return Some(Msg::Job(JobMsg::GetDetails(job.id)));
                            }
                            CmdResult::None
                        }
                        MouseEventKind::ScrollUp => self.perform(Cmd::Move(Direction::Up)),
                        MouseEventKind::ScrollDown => self.perform(Cmd::Move(Direction::Down)),
                        _ => CmdResult::None,
                    }
                }
            }
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
