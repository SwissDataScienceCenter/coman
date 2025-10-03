use color_eyre::Result;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    app::{Mode, SubMode},
    components::Component,
    config::Config,
};
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::{prelude::*, widgets::*};

#[derive(Clone)]
struct MenuItem<'a> {
    content: Text<'a>,
    action: Action,
}
impl<'a> MenuItem<'a> {
    fn new<T>(content: T, action: Action) -> Self
    where
        T: Into<Text<'a>>,
    {
        Self {
            content: content.into(),
            action,
        }
    }
}

impl<'a> From<MenuItem<'a>> for ListItem<'a> {
    fn from(value: MenuItem<'a>) -> Self {
        ListItem::new(value.content.clone())
    }
}

#[derive(Default)]
pub struct WorkloadListMenu<'a> {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    mode: Mode,
    sub_mode: SubMode,
    state: ListState,
    last_area: Rect,
    items: Vec<MenuItem<'a>>,
}

impl<'a> WorkloadListMenu<'a> {
    pub fn new() -> Self {
        Self {
            last_area: Rect::ZERO,
            items: vec![
                MenuItem::new("Login to CSCS", Action::CSCSLogin),
                MenuItem::new("todo", Action::Escape),
            ],
            ..Self::default()
        }
    }
    fn show(&self) -> bool {
        match self.sub_mode {
            SubMode::Main => false,
            SubMode::Menu => true,
        }
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
}

impl<'a> Component for WorkloadListMenu<'a> {
    fn register_action_handler(
        &mut self,
        tx: UnboundedSender<Action>,
    ) -> color_eyre::eyre::Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> color_eyre::eyre::Result<()> {
        self.config = config;
        Ok(())
    }

    fn handle_mouse_event(
        &mut self,
        mouse: crossterm::event::MouseEvent,
    ) -> color_eyre::eyre::Result<Option<Action>> {
        let _ = mouse; // to appease clippy
        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::eyre::Result<Option<Action>> {
        match action {
            Action::Tick => {
                // add any logic here that should run on every tick
            }
            Action::Render => {
                // add any logic here that should run on every render
            }
            Action::Mode(mode) => {
                self.mode = mode;
            }
            Action::SubMode(sub_mode) => {
                self.sub_mode = sub_mode;
                if self.sub_mode == SubMode::Menu {
                    self.select_first();
                }
            }
            Action::Up => {
                if self.show() {
                    self.select_previous();
                }
            }
            Action::Down => {
                if self.show() {
                    self.select_next();
                }
            }
            Action::Enter => {
                if let Some(index) = self.state.selected() {
                    self.command_tx
                        .as_mut()
                        .map(|tx| tx.send(self.items[index].action.clone()));
                    return Ok(Some(Action::Menu));
                }
            }
            _ => {}
        }
        Ok(None)
    }
    fn draw(
        &mut self,
        frame: &mut ratatui::Frame,
        area: ratatui::prelude::Rect,
    ) -> color_eyre::eyre::Result<()> {
        if !self.show() {
            return Ok(());
        }
        self.last_area = area;

        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Max(100), Constraint::Min(5)])
            .split(area);

        let inner = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(5), Constraint::Max(100), Constraint::Min(5)])
            .split(outer[1]);
        let menu = Block::bordered().title("Menu").padding(Padding::uniform(2));

        let options = List::new(self.items.clone())
            .block(menu)
            .highlight_style(Style::new().bg(Color::DarkGray))
            .highlight_symbol("> ");
        frame.render_stateful_widget(options, inner[1], &mut self.state);
        Ok(())
    }
}
