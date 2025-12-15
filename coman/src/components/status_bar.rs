use std::time::{Duration, Instant};

use ratatui::{
    text::{Line, Span},
    widgets::{LineGauge, Paragraph},
};
use tuirealm::{
    AttrValue, Attribute, Component, Event, MockComponent, Props, State,
    command::CmdResult,
    props::{BorderType, Borders, Layout},
    ratatui::{
        Frame,
        layout::{Constraint, Direction},
        prelude::Rect,
        style::{Color, Style},
        widgets::Block,
    },
};

use crate::{
    app::{
        messages::Msg,
        user_events::{CscsEvent, StatusEvent, UserEvent},
    },
    config::Config,
};

pub struct StatusBar {
    props: Props,
    last_updated: Instant,
    current_status: Option<StatusEvent>,
    status_clear_time: Duration,
    current_platform: String,
    current_system: String,
}

impl StatusBar {
    pub fn new() -> Self {
        let config = Config::new().unwrap();
        Self {
            props: Props::default(),
            last_updated: Instant::now(),
            current_status: None,
            status_clear_time: Duration::from_secs(10),
            current_platform: config.values.cscs.current_platform.to_string(),
            current_system: config.values.cscs.current_system,
        }
    }
}

impl MockComponent for StatusBar {
    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }

    fn state(&self) -> State {
        State::None
    }

    fn perform(&mut self, _cmd: tuirealm::command::Cmd) -> CmdResult {
        CmdResult::None
    }
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        if self.props.get_or(Attribute::Display, AttrValue::Flag(true)) == AttrValue::Flag(true) {
            let borders = Borders::default().modifiers(BorderType::Rounded);
            let div = Block::default()
                .borders(borders.sides)
                .border_style(borders.style())
                .border_type(borders.modifiers);
            let layout = Layout::default()
                .constraints(&[Constraint::Min(34), Constraint::Fill(1)])
                .direction(Direction::Horizontal)
                .margin(1);
            frame.render_widget(div, area);

            let highlight_style = Style::default().fg(Color::Yellow);
            let info_style = Style::default().fg(Color::Blue);
            let warn_style = Style::default().fg(Color::Red);
            let system_status = Paragraph::new(Line::from(vec![
                Span::styled("Platform: ", highlight_style),
                Span::raw(self.current_platform.clone().to_uppercase()),
                Span::raw(" "),
                Span::styled("System: ", highlight_style),
                Span::raw(self.current_system.clone()),
            ]));
            let chunks = layout.chunks(area);
            frame.render_widget(system_status, chunks[0]);
            if let Some(status) = self.current_status.clone() {
                match status {
                    StatusEvent::Progress(msg, progress) => {
                        let gauge = LineGauge::default()
                            .filled_style(Style::default().fg(Color::DarkGray))
                            .label(msg)
                            .ratio((progress as f64) / 100.0);
                        frame.render_widget(gauge, chunks[1]);
                    }
                    StatusEvent::Info(info) => {
                        let notification_status = Paragraph::new(info).style(info_style);
                        frame.render_widget(notification_status, chunks[1]);
                    }
                    StatusEvent::Warning(warning) => {
                        let notification_status = Paragraph::new(warning).style(warn_style);
                        frame.render_widget(notification_status, chunks[1]);
                    }
                }
            }
        }
    }
}
impl Component<Msg, UserEvent> for StatusBar {
    fn on(&mut self, ev: Event<UserEvent>) -> Option<Msg> {
        match ev {
            Event::Tick => {
                if self.last_updated.elapsed() > self.status_clear_time {
                    self.current_status = None;
                }
            }
            Event::User(UserEvent::Status(status)) => {
                self.current_status = Some(status);
                self.last_updated = Instant::now();
            }
            Event::User(UserEvent::Cscs(CscsEvent::SystemSelected(system))) => {
                self.current_system = system;
            }
            _ => {}
        }
        None
    }
}
