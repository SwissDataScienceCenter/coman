use cli_log::*;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent},
    prelude::*,
    style::palette::tailwind::*,
    widgets::{Block, Borders, List, ListItem, ListState},
};
pub struct WorkloadList<'a> {
    pub title: String,
    pub focus: bool,
    pub items: Vec<WorkloadItem>,
    pub state: ListState,
    last_area: Rect,
    list_items: Vec<ListItem<'a>>,
}

impl WorkloadList<'_> {
    pub fn new(title: &str) -> Self {
        WorkloadList {
            title: title.to_string(),
            items: vec![],
            focus: false,
            state: ListState::default(),
            last_area: Rect::ZERO,
            list_items: vec![],
        }
    }
    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }
    pub fn items<I, T>(mut self, items: T) -> Self
    where
        T: IntoIterator<Item = I>,
        WorkloadItem: From<I>,
    {
        self.items = items.into_iter().map(|i| WorkloadItem::from(i)).collect();
        self
    }

    pub fn select_none(&mut self) {
        self.state.select(None);
    }
    pub fn select_next(&mut self) {
        self.state.select_next();
    }
    pub fn select_previous(&mut self) {
        self.state.select_previous();
    }
    pub fn select_first(&mut self) {
        self.state.select_first();
    }
    pub fn select_last(&mut self) {
        self.state.select_last();
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(k) => {
                self.handle_key(k);
            }
            Event::Mouse(m) => {
                self.handle_mouse(m);
            }
            Event::FocusGained => todo!(),
            Event::FocusLost => todo!(),
            Event::Paste(_) => {}
            Event::Resize(_, _) => {}
        }
    }
    fn handle_key(&mut self, event: KeyEvent) {
        if !self.focus {
            return;
        }
        if event.kind != KeyEventKind::Press {
            return;
        }
        match event.code {
            KeyCode::Up | KeyCode::Char('k') => self.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            _ => {}
        }
    }
    fn handle_mouse(&mut self, event: MouseEvent) {
        match event.kind {
            ratatui::crossterm::event::MouseEventKind::Up(mouse_button) => match mouse_button {
                ratatui::crossterm::event::MouseButton::Left => {
                    if !self
                        .last_area
                        .contains(Position::new(event.column, event.row))
                    {
                        self.focus = false;
                        return;
                    }
                    self.focus = true;
                    let index =
                        (event.row - self.last_area.top() - 1) as usize + self.state.offset();
                    self.state.select(Some(index));
                }
                ratatui::crossterm::event::MouseButton::Right => todo!(),
                _ => {}
            },
            ratatui::crossterm::event::MouseEventKind::ScrollDown => {
                if self
                    .last_area
                    .contains(Position::new(event.column, event.row))
                {
                    self.state.scroll_down_by(1)
                }
            }
            ratatui::crossterm::event::MouseEventKind::ScrollUp => {
                if self
                    .last_area
                    .contains(Position::new(event.column, event.row))
                {
                    self.state.scroll_up_by(1)
                }
            }
            _ => {}
        }
    }
}

#[derive(Debug)]
pub struct WorkloadItem {
    pub name: String,
    pub status: WorkloadStatus,
}
impl WorkloadItem {
    fn new(status: WorkloadStatus, name: &str) -> Self {
        Self {
            name: name.to_string(),
            status,
        }
    }
}
impl From<(WorkloadStatus, &'static str)> for WorkloadItem {
    fn from(value: (WorkloadStatus, &'static str)) -> WorkloadItem {
        WorkloadItem {
            name: value.1.to_string(),
            status: value.0,
        }
    }
}

impl From<&WorkloadItem> for ListItem<'_> {
    fn from(value: &WorkloadItem) -> Self {
        let line = match value.status {
            WorkloadStatus::Starting => Line::styled(value.name.clone(), Color::Green),
            WorkloadStatus::Running => Line::styled(value.name.clone(), Color::Blue),
            WorkloadStatus::Terminating => Line::styled(value.name.clone(), Color::Yellow),
            WorkloadStatus::Stopped => Line::styled(value.name.clone(), Color::Gray),
            WorkloadStatus::Failed => Line::styled(value.name.clone(), Color::Red),
        };
        ListItem::new(line)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorkloadStatus {
    Starting,
    Running,
    Terminating,
    Stopped,
    Failed,
}
impl Widget for &mut WorkloadList<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem<'_>> = self.items.iter().map(ListItem::from).collect();

        let block = Block::new()
            .title(Line::raw(self.title.clone()).centered())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(Style::new().fg(SLATE.c100).bg(BLUE.c800))
            .bg(SLATE.c950);

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::new().bg(Color::DarkGray))
            .highlight_symbol("> ");
        self.last_area = area;
        StatefulWidget::render(list, area, buf, &mut self.state);
    }
}
