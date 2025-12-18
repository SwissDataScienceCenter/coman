use tuirealm::{
    AttrValue, Attribute, Component, Event, MockComponent, Props, State,
    command::{Cmd, CmdResult},
    event::{Key, KeyEvent, KeyModifiers},
    props::{BorderType, Borders, Layout},
    ratatui::{
        layout::{Constraint, Direction},
        style::{Color, Style},
        widgets::{Block, Paragraph},
    },
};

use crate::{
    app::{
        messages::{JobMsg, Msg},
        user_events::UserEvent,
    },
    cscs::api_client::types::JobDetail,
};

pub struct WorkloadDetails {
    props: Props,
    details: JobDetail,
}

impl WorkloadDetails {
    pub fn new(details: JobDetail) -> Self {
        Self {
            props: Props::default(),
            details,
        }
    }
}

impl MockComponent for WorkloadDetails {
    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }

    fn state(&self) -> tuirealm::State {
        State::None
    }

    fn perform(&mut self, _cmd: Cmd) -> CmdResult {
        CmdResult::None
    }
    fn view(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        if self.props.get_or(Attribute::Display, AttrValue::Flag(true)) == AttrValue::Flag(true) {
            let borders = Borders::default().modifiers(BorderType::Rounded);
            let div = Block::default()
                .borders(borders.sides)
                .border_style(borders.style())
                .border_type(borders.modifiers)
                .title(format!("Job Details {}", self.details.name));
            frame.render_widget(div, area);
            let row_constraints: Vec<Constraint> = (0..7).map(|_| Constraint::Length(2)).collect();

            let vertical = Layout::default()
                .constraints(&row_constraints)
                .direction(Direction::Vertical)
                .margin(1);
            let horizontal = Layout::default()
                .constraints(&[Constraint::Length(10), Constraint::Fill(1)])
                .direction(Direction::Horizontal);

            let chunks = vertical.chunks(area);

            let label_style = Style::default().fg(Color::Yellow);
            let row_chunks = horizontal.chunks(chunks[0]);
            frame.render_widget(Paragraph::new("Name").style(label_style), row_chunks[0]);
            frame.render_widget(Paragraph::new(self.details.name.clone()), row_chunks[1]);

            let row_chunks = horizontal.chunks(chunks[1]);
            frame.render_widget(Paragraph::new("Id").style(label_style), row_chunks[0]);
            frame.render_widget(Paragraph::new(self.details.id.to_string()), row_chunks[1]);

            let row_chunks = horizontal.chunks(chunks[2]);
            frame.render_widget(Paragraph::new("Start").style(label_style), row_chunks[0]);
            frame.render_widget(
                Paragraph::new(self.details.start_date.map(|d| d.to_string()).unwrap_or("".to_owned())),
                row_chunks[1],
            );

            let row_chunks = horizontal.chunks(chunks[3]);
            frame.render_widget(Paragraph::new("End").style(label_style), row_chunks[0]);
            frame.render_widget(
                Paragraph::new(self.details.end_date.map(|d| d.to_string()).unwrap_or("".to_owned())),
                row_chunks[1],
            );

            let row_chunks = horizontal.chunks(chunks[4]);
            frame.render_widget(Paragraph::new("Status").style(label_style), row_chunks[0]);
            frame.render_widget(Paragraph::new(self.details.status.to_string()), row_chunks[1]);

            let row_chunks = horizontal.chunks(chunks[5]);
            frame.render_widget(Paragraph::new("Exit Code").style(label_style), row_chunks[0]);
            frame.render_widget(Paragraph::new(self.details.exit_code.to_string()), row_chunks[1]);

            let row_chunks = horizontal.chunks(chunks[6]);
            frame.render_widget(Paragraph::new("User").style(label_style), row_chunks[0]);
            frame.render_widget(Paragraph::new(self.details.user.to_string()), row_chunks[1]);
        }
    }
}

impl Component<Msg, UserEvent> for WorkloadDetails {
    fn on(&mut self, ev: Event<UserEvent>) -> Option<Msg> {
        let _ = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('l'),
                modifiers: KeyModifiers::NONE,
            }) => {
                return Some(Msg::Job(JobMsg::Log(self.details.id)));
            }
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => {
                return Some(Msg::Job(JobMsg::Close));
            }
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
