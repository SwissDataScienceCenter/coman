use color_eyre::eyre::eyre;
use itertools::Itertools;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    app::Mode,
    components::{Component, button::Button},
    config::Config,
    trace_dbg,
};
use ratatui::{prelude::*, widgets::*};

#[derive(Default)]
pub struct Popup {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    title: Option<String>,
    content: Option<String>,
    button: Button,
    show: bool,
}

impl Popup {
    pub fn new() -> Self {
        Self {
            button: Button::new("Ok".to_string()).on_click(Action::ClosePopup),
            ..Default::default()
        }
    }
}

impl Component for Popup {
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

    fn handle_events(
        &mut self,
        event: Option<crate::tui::Event>,
    ) -> color_eyre::eyre::Result<Option<Action>> {
        let action = match event {
            Some(crate::tui::Event::Key(key_event)) => self.handle_key_event(key_event)?,
            Some(crate::tui::Event::Mouse(mouse_event)) => self.handle_mouse_event(mouse_event)?,
            _ => None,
        };
        Ok(action)
    }

    fn handle_key_event(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> color_eyre::eyre::Result<Option<Action>> {
        let _ = key; // to appease clippy
        Ok(None)
    }

    fn handle_mouse_event(
        &mut self,
        mouse: crossterm::event::MouseEvent,
    ) -> color_eyre::eyre::Result<Option<Action>> {
        self.button.handle_mouse_event(mouse)
    }

    fn update(&mut self, action: Action) -> color_eyre::eyre::Result<Option<Action>> {
        match action {
            Action::Tick => {
                // add any logic here that should run on every tick
            }
            Action::Render => {
                // add any logic here that should run on every render
            }
            Action::ClosePopup | Action::Enter | Action::Escape => {
                self.show = false;
            }
            Action::Error(e) => {
                self.show = true;
                self.content = Some(e.message);
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
        if self.show {
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(5), Constraint::Min(20), Constraint::Min(5)])
                .split(area);
            let inner_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(5), Constraint::Min(20), Constraint::Min(5)])
                .split(layout[1]);
            let block = Block::bordered().gray().title(
                self.title
                    .clone()
                    .unwrap_or("".to_string())
                    .bold()
                    .into_centered_line(),
            );
            let block_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(5), Constraint::Length(3)])
                .split(block.inner(inner_layout[1]));
            frame.render_widget(
                Line::raw(self.content.clone().unwrap_or("".to_string())),
                block_layout[0],
            );
            frame.render_widget(&mut self.button, block_layout[1]);

            frame.render_widget(block, inner_layout[1]);
        }
        Ok(())
    }
}
