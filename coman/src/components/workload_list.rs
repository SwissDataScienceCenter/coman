use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{action::Action, config::Config};

#[derive(Default)]
pub struct WorkloadList<'a> {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    state: ListState,
    last_area: Rect,
    list_items: Vec<ListItem<'a>>,
    id: String,
    focus: bool,
}

#[allow(dead_code)]
impl<'a> WorkloadList<'a> {
    pub fn new(id: String) -> Self {
        Self {
            last_area: Rect::ZERO,
            id,
            ..Self::default()
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

impl<'a> Component for WorkloadList<'a> {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                // add any logic here that should run on every tick
            }
            Action::Render => {
                // add any logic here that should run on every render
            }
            Action::FocusChanged(component_id, _) => {
                if component_id == self.id {
                    self.focus = true;
                } else {
                    self.focus = false;
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        self.last_area = area;
        frame.render_widget(Paragraph::new("hello world"), area);
        Ok(())
    }
}
