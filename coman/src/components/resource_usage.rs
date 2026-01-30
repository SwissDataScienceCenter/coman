use bytesize::ByteSize;
use chrono::{Local, TimeDelta};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction as LayoutDirection, Rect},
    symbols::Marker,
    widgets::GraphType,
};
use tui_realm_stdlib::{
    Chart, Container,
    props::{CHART_X_BOUNDS, CHART_X_LABELS, CHART_Y_BOUNDS, CHART_Y_LABELS},
};
use tuirealm::{
    AttrValue, Attribute, Component, Event, MockComponent, State,
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent},
    props::{Borders, Color, Dataset, Layout, PropPayload, PropValue, Style},
};

use crate::app::{
    messages::{JobMsg, Msg},
    user_events::{CscsEvent, UserEvent},
};
pub const UPDATE_CPU_DATA: &str = "update-cpu-data";
pub const UPDATE_MEMORY_DATA: &str = "update-memory-data";
pub const UPDATE_GPU_DATA: &str = "update-gpu-data";

#[derive(MockComponent)]
pub struct ResourceUsage {
    component: Container,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            component: Container::default()
                .title("Resource Usage", Alignment::Left)
                .layout(
                    Layout::default()
                        .constraints(&[
                            Constraint::Percentage(33),
                            Constraint::Percentage(33),
                            Constraint::Percentage(34),
                        ])
                        .direction(LayoutDirection::Vertical)
                        .margin(2),
                )
                .children(vec![
                    Box::new(CpuUsage::default()),
                    Box::new(MemoryUsage::default()),
                    Box::new(GpuUsage::default()),
                ]),
        }
    }
}

impl Component<Msg, UserEvent> for ResourceUsage {
    fn on(&mut self, ev: Event<UserEvent>) -> Option<Msg> {
        let _ = match ev {
            Event::Keyboard(KeyEvent { code: Key::Left, .. }) => self.perform(Cmd::Move(Direction::Left)),
            Event::Keyboard(KeyEvent { code: Key::Right, .. }) => self.perform(Cmd::Move(Direction::Right)),
            Event::Keyboard(KeyEvent { code: Key::Home, .. }) => self.perform(Cmd::GoTo(Position::Begin)),
            Event::Keyboard(KeyEvent { code: Key::End, .. }) => self.perform(Cmd::GoTo(Position::End)),
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => {
                return Some(Msg::Job(JobMsg::Close));
            }
            Event::User(UserEvent::Cscs(CscsEvent::GotJobResourceUsage(ru))) => {
                self.attr(
                    Attribute::Custom(UPDATE_CPU_DATA),
                    AttrValue::Payload(PropPayload::One(PropValue::F64(ru.cpu as f64))),
                );
                self.attr(
                    Attribute::Custom(UPDATE_MEMORY_DATA),
                    AttrValue::Payload(PropPayload::Tup2((PropValue::U64(ru.rss), PropValue::U64(ru.vsz)))),
                );
                if let Some(gpu) = ru.gpu {
                    self.attr(
                        Attribute::Custom(UPDATE_GPU_DATA),
                        AttrValue::Payload(PropPayload::Vec(gpu.iter().map(|g| PropValue::U64(g.1)).collect())),
                    );
                }
                CmdResult::None
            }
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}

// START CPU
struct CpuUsage {
    component: Chart,
    dataset: Dataset,
    max_y: f64,
}

impl Default for CpuUsage {
    fn default() -> Self {
        let current_time = Local::now();
        let cur_time_str = current_time.format("%H:%M:%S").to_string();
        let start = current_time.checked_sub_signed(TimeDelta::minutes(5)).unwrap();
        let start_str = start.format("%H:%M:%S").to_string();
        Self {
            component: Chart::default()
                .disabled(false)
                .title("CPU", Alignment::Left)
                .borders(Borders::default())
                .x_style(Style::default().fg(Color::LightBlue))
                .x_title("")
                .x_labels(&[&start_str, &cur_time_str])
                .x_bounds((start.timestamp() as f64, current_time.timestamp() as f64))
                .y_style(Style::default().fg(Color::Yellow))
                .y_title("")
                .y_bounds((0.0, 1.0))
                .y_labels(&["0%", "100%"]),
            dataset: Dataset::default()
                .name("CPU")
                .graph_type(GraphType::Line)
                .marker(Marker::Braille)
                .style(Style::default().fg(Color::Cyan))
                .data(Vec::new()),
            max_y: 1.0,
        }
    }
}
impl MockComponent for CpuUsage {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.component.view(frame, area);
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.component.query(attr)
    }

    fn attr(&mut self, query: Attribute, attr: AttrValue) {
        match query {
            Attribute::Custom(UPDATE_CPU_DATA) => {
                // Update data
                let mut current_data = self.dataset.get_data().to_vec();
                // update data
                let current_time = Local::now();
                let since_epoch = current_time.timestamp() as f64;
                let cpu_usage = attr.unwrap_payload().unwrap_one().unwrap_f64();
                if cpu_usage > self.max_y {
                    self.max_y = cpu_usage;
                    self.attr(
                        Attribute::Custom(CHART_Y_BOUNDS),
                        AttrValue::Payload(PropPayload::Tup2((PropValue::F64(0.0), PropValue::F64(self.max_y)))),
                    );
                }
                current_data.push((since_epoch, cpu_usage));
                self.dataset = self.dataset.clone().data(current_data);
                // update bounds
                let start_time = self
                    .query(Attribute::Custom(CHART_X_BOUNDS))
                    .unwrap()
                    .unwrap_payload()
                    .unwrap_tup2()
                    .0
                    .unwrap_f64();
                self.attr(
                    Attribute::Custom(CHART_X_BOUNDS),
                    AttrValue::Payload(PropPayload::Tup2((
                        PropValue::F64(start_time),
                        PropValue::F64(since_epoch),
                    ))),
                );
                //update labels
                let labels = self
                    .query(Attribute::Custom(CHART_X_LABELS))
                    .unwrap()
                    .unwrap_payload()
                    .unwrap_vec();
                let start_label = labels[0].clone();
                self.attr(
                    Attribute::Custom(CHART_X_LABELS),
                    AttrValue::Payload(PropPayload::Vec(vec![
                        start_label,
                        PropValue::Str(current_time.format("%H:%M:%S").to_string()),
                    ])),
                );

                let labels = self
                    .query(Attribute::Custom(CHART_Y_LABELS))
                    .unwrap()
                    .unwrap_payload()
                    .unwrap_vec();
                let start_label = labels[0].clone();
                self.attr(
                    Attribute::Custom(CHART_Y_LABELS),
                    AttrValue::Payload(PropPayload::Vec(vec![
                        start_label,
                        PropValue::Str(format!("{:.0}%", self.max_y.ceil())),
                    ])),
                );

                self.attr(
                    Attribute::Dataset,
                    AttrValue::Payload(PropPayload::Vec(vec![PropValue::Dataset(self.dataset.clone())])),
                );
            }
            _ => self.component.attr(query, attr),
        }
    }

    fn state(&self) -> State {
        self.component.state()
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        self.component.perform(cmd)
    }
}

impl Component<Msg, UserEvent> for CpuUsage {
    fn on(&mut self, _ev: Event<UserEvent>) -> Option<Msg> {
        Some(Msg::None)
    }
}

// START MEMORY

struct MemoryUsage {
    component: Chart,
    rss_dataset: Dataset,
    vsz_dataset: Dataset,
    max_y: u64,
}

impl Default for MemoryUsage {
    fn default() -> Self {
        let current_time = Local::now();
        let cur_time_str = current_time.format("%H:%M:%S").to_string();
        let start = current_time.checked_sub_signed(TimeDelta::minutes(5)).unwrap();
        let start_str = start.format("%H:%M:%S").to_string();
        Self {
            component: Chart::default()
                .disabled(false)
                .title("Memory", Alignment::Left)
                .borders(Borders::default())
                .x_style(Style::default().fg(Color::LightBlue))
                .x_title("")
                .x_labels(&[&start_str, &cur_time_str])
                .x_bounds((start.timestamp() as f64, current_time.timestamp() as f64))
                .y_style(Style::default().fg(Color::Yellow))
                .y_title("")
                .y_bounds((0.0, 1.0))
                .y_labels(&["0", "1"]),
            rss_dataset: Dataset::default()
                .name("RSS")
                .graph_type(GraphType::Line)
                .marker(Marker::Braille)
                .style(Style::default().fg(Color::Cyan))
                .data(Vec::new()),
            vsz_dataset: Dataset::default()
                .name("VSZ")
                .graph_type(GraphType::Line)
                .marker(Marker::Braille)
                .style(Style::default().fg(Color::Green))
                .data(Vec::new()),
            max_y: 1,
        }
    }
}
impl MockComponent for MemoryUsage {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.component.view(frame, area);
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.component.query(attr)
    }

    fn attr(&mut self, query: Attribute, attr: AttrValue) {
        match query {
            Attribute::Custom(UPDATE_MEMORY_DATA) => {
                // Update data
                let mut current_rss_data = self.rss_dataset.get_data().to_vec();
                // update data
                let current_time = Local::now();
                let since_epoch = current_time.timestamp() as f64;
                let usage = attr.unwrap_payload().unwrap_tup2();

                let rss_usage = usage.0.unwrap_u64();
                if rss_usage > self.max_y {
                    self.max_y = rss_usage;
                    self.attr(
                        Attribute::Custom(CHART_Y_BOUNDS),
                        AttrValue::Payload(PropPayload::Tup2((
                            PropValue::F64(0.0),
                            PropValue::F64(self.max_y as f64),
                        ))),
                    );
                }
                current_rss_data.push((since_epoch, rss_usage as f64));
                self.rss_dataset = self.rss_dataset.clone().data(current_rss_data);

                let mut current_vsz_data = self.vsz_dataset.get_data().to_vec();
                let vsz_usage = usage.1.unwrap_u64();
                if vsz_usage > self.max_y {
                    self.max_y = vsz_usage;
                    self.attr(
                        Attribute::Custom(CHART_Y_BOUNDS),
                        AttrValue::Payload(PropPayload::Tup2((
                            PropValue::F64(0.0),
                            PropValue::F64(self.max_y as f64),
                        ))),
                    );
                }
                current_vsz_data.push((since_epoch, vsz_usage as f64));
                self.vsz_dataset = self.vsz_dataset.clone().data(current_vsz_data);
                // update bounds
                let start_time = self
                    .query(Attribute::Custom(CHART_X_BOUNDS))
                    .unwrap()
                    .unwrap_payload()
                    .unwrap_tup2()
                    .0
                    .unwrap_f64();
                self.attr(
                    Attribute::Custom(CHART_X_BOUNDS),
                    AttrValue::Payload(PropPayload::Tup2((
                        PropValue::F64(start_time),
                        PropValue::F64(since_epoch),
                    ))),
                );
                //update labels
                let labels = self
                    .query(Attribute::Custom(CHART_X_LABELS))
                    .unwrap()
                    .unwrap_payload()
                    .unwrap_vec();
                let start_label = labels[0].clone();
                self.attr(
                    Attribute::Custom(CHART_X_LABELS),
                    AttrValue::Payload(PropPayload::Vec(vec![
                        start_label,
                        PropValue::Str(current_time.format("%H:%M:%S").to_string()),
                    ])),
                );

                let labels = self
                    .query(Attribute::Custom(CHART_Y_LABELS))
                    .unwrap()
                    .unwrap_payload()
                    .unwrap_vec();
                let start_label = labels[0].clone();
                self.attr(
                    Attribute::Custom(CHART_Y_LABELS),
                    AttrValue::Payload(PropPayload::Vec(vec![
                        start_label,
                        PropValue::Str(ByteSize::b(self.max_y).display().iec().to_string()),
                    ])),
                );

                self.attr(
                    Attribute::Dataset,
                    AttrValue::Payload(PropPayload::Vec(vec![
                        PropValue::Dataset(self.rss_dataset.clone()),
                        PropValue::Dataset(self.vsz_dataset.clone()),
                    ])),
                );
            }
            _ => self.component.attr(query, attr),
        }
    }

    fn state(&self) -> State {
        self.component.state()
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        self.component.perform(cmd)
    }
}

impl Component<Msg, UserEvent> for MemoryUsage {
    fn on(&mut self, _ev: Event<UserEvent>) -> Option<Msg> {
        Some(Msg::None)
    }
}

// START GPU

const PALETTE: [Color; 8] = [
    Color::Cyan,
    Color::Green,
    Color::Red,
    Color::Blue,
    Color::Magenta,
    Color::Yellow,
    Color::Gray,
    Color::LightYellow,
];
struct GpuUsage {
    component: Chart,
    datasets: Vec<Dataset>,
    max_y: u64,
}

impl Default for GpuUsage {
    fn default() -> Self {
        let current_time = Local::now();
        let cur_time_str = current_time.format("%H:%M:%S").to_string();
        let start = current_time.checked_sub_signed(TimeDelta::minutes(5)).unwrap();
        let start_str = start.format("%H:%M:%S").to_string();
        Self {
            component: Chart::default()
                .disabled(false)
                .title("GPU", Alignment::Left)
                .borders(Borders::default())
                .x_style(Style::default().fg(Color::LightBlue))
                .x_title("")
                .x_labels(&[&start_str, &cur_time_str])
                .x_bounds((start.timestamp() as f64, current_time.timestamp() as f64))
                .y_style(Style::default().fg(Color::Yellow))
                .y_title("")
                .y_bounds((0.0, 1.0))
                .y_labels(&["0", "1"]),
            datasets: Vec::new(),
            max_y: 1,
        }
    }
}
impl MockComponent for GpuUsage {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.component.view(frame, area);
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.component.query(attr)
    }

    fn attr(&mut self, query: Attribute, attr: AttrValue) {
        match query {
            Attribute::Custom(UPDATE_GPU_DATA) => {
                let payload = attr.unwrap_payload().unwrap_vec();
                let current_time = Local::now();
                let since_epoch = current_time.timestamp() as f64;
                for (i, gpu_payload) in payload.iter().enumerate() {
                    // Update data
                    let mut current_data = self
                        .datasets
                        .get(i)
                        .map(|d| d.get_data().to_vec())
                        .unwrap_or(Vec::new());
                    // update data
                    let gpu_usage = gpu_payload.clone().unwrap_u64();
                    if gpu_usage > self.max_y {
                        self.max_y = gpu_usage;
                        self.attr(
                            Attribute::Custom(CHART_Y_BOUNDS),
                            AttrValue::Payload(PropPayload::Tup2((
                                PropValue::F64(0.0),
                                PropValue::F64(self.max_y as f64),
                            ))),
                        );
                    }
                    current_data.push((since_epoch, gpu_usage as f64));
                    match self.datasets.get_mut(i) {
                        Some(ds) => {
                            self.datasets[i] = ds.clone().data(current_data);
                        }
                        None => self.datasets.push(
                            Dataset::default()
                                .name(format!("GPU {i}"))
                                .graph_type(GraphType::Line)
                                .marker(Marker::Braille)
                                .style(Style::default().fg(PALETTE[i % 8]))
                                .data(current_data),
                        ),
                    }
                }
                // update bounds
                let start_time = self
                    .query(Attribute::Custom(CHART_X_BOUNDS))
                    .unwrap()
                    .unwrap_payload()
                    .unwrap_tup2()
                    .0
                    .unwrap_f64();
                self.attr(
                    Attribute::Custom(CHART_X_BOUNDS),
                    AttrValue::Payload(PropPayload::Tup2((
                        PropValue::F64(start_time),
                        PropValue::F64(since_epoch),
                    ))),
                );
                //update labels
                let labels = self
                    .query(Attribute::Custom(CHART_X_LABELS))
                    .unwrap()
                    .unwrap_payload()
                    .unwrap_vec();
                let start_label = labels[0].clone();
                self.attr(
                    Attribute::Custom(CHART_X_LABELS),
                    AttrValue::Payload(PropPayload::Vec(vec![
                        start_label,
                        PropValue::Str(current_time.format("%H:%M:%S").to_string()),
                    ])),
                );

                let labels = self
                    .query(Attribute::Custom(CHART_Y_LABELS))
                    .unwrap()
                    .unwrap_payload()
                    .unwrap_vec();
                let start_label = labels[0].clone();
                self.attr(
                    Attribute::Custom(CHART_Y_LABELS),
                    AttrValue::Payload(PropPayload::Vec(vec![
                        start_label,
                        PropValue::Str(ByteSize::b(self.max_y).display().iec().to_string()),
                    ])),
                );

                self.attr(
                    Attribute::Dataset,
                    AttrValue::Payload(PropPayload::Vec(
                        self.datasets.iter().map(|d| PropValue::Dataset(d.clone())).collect(),
                    )),
                );
            }
            _ => self.component.attr(query, attr),
        }
    }

    fn state(&self) -> State {
        self.component.state()
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        self.component.perform(cmd)
    }
}

impl Component<Msg, UserEvent> for GpuUsage {
    fn on(&mut self, _ev: Event<UserEvent>) -> Option<Msg> {
        Some(Msg::None)
    }
}
