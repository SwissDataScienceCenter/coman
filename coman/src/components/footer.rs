use color_eyre::eyre::eyre;
use itertools::Itertools;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    app::Mode,
    components::{Component, workload_menu::WorkloadListMenu},
    config::Config,
    focus_manager::Focus,
};
use ratatui::{prelude::*, widgets::*};

pub struct Footer {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    mode: Mode,
    id: String,
    menu: Box<dyn Component>,
    focus: Focus,
}

impl Footer {
    pub fn new(id: String) -> Self {
        Self {
            id,
            menu: Box::new(WorkloadListMenu::new("WorkloadListMenu".to_string())),
            command_tx: None,
            config: Config::default(),
            mode: Mode::Main,
            focus: Focus::Inactive,
        }
    }
}

impl Component for Footer {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn register_action_handler(
        &mut self,
        tx: UnboundedSender<Action>,
    ) -> color_eyre::eyre::Result<()> {
        self.command_tx = Some(tx.clone());
        self.menu.register_action_handler(tx)?;
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
        let _ = mouse; // to appease clippy
        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::eyre::Result<Option<Action>> {
        match action.clone() {
            Action::FocusChanged(component_id, focus) => {
                if component_id == self.id {
                    self.focus = focus;
                } else {
                    match (focus, self.focus.clone()) {
                        (Focus::Active, Focus::Active) => self.focus = Focus::Inactive,
                        (Focus::Active, Focus::PermanentInactive) => self.focus = Focus::Permanent, // focus changes to active means any exclusive focus has ended
                        (Focus::Active, Focus::Exclusive) => {
                            self.focus = Focus::Inactive;
                        }
                        (Focus::Exclusive, Focus::Active) => self.focus = Focus::Inactive,
                        (Focus::Exclusive, Focus::Permanent) => {
                            self.focus = Focus::PermanentInactive
                        }
                        (Focus::Exclusive, Focus::Exclusive) => self.focus = Focus::Inactive,
                        _ => {}
                    }
                }
            }
            Action::Mode(mode) => {
                self.mode = mode;
            }
            _ => {}
        };
        // forward events to menu
        self.menu.update(action)
    }
    fn draw(
        &mut self,
        frame: &mut ratatui::Frame,
        area: ratatui::prelude::Rect,
    ) -> color_eyre::eyre::Result<()> {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(2)])
            .split(area);
        let keybindings = self.config.keybindings.0[&self.mode].clone();
        let footer = match self.mode {
            Mode::Main => {
                // get text for current key bindings
                let quit_key = keybindings
                    .clone()
                    .iter()
                    .filter(|(_, v)| **v == Action::Quit)
                    .flat_map(|(k, _)| k)
                    .map(|k| {
                        if k.modifiers.is_empty() {
                            k.code.to_string()
                        } else {
                            format!("{}-{}", k.modifiers, k.code)
                        }
                    })
                    .sorted_by_key(|k| k.chars().count())
                    .next()
                    .ok_or(eyre!("no quit key defined"))?;
                let menu_key = keybindings
                    .clone()
                    .iter()
                    .filter(|(_, v)| **v == Action::Menu)
                    .flat_map(|(k, _)| k)
                    .map(|k| {
                        if k.modifiers.is_empty() {
                            k.code.to_string()
                        } else {
                            format!("{}-{}", k.modifiers, k.code)
                        }
                    })
                    .sorted_by_key(|k| k.chars().count())
                    .next()
                    .ok_or(eyre!("no menu key defined"))?;

                Paragraph::new(format!("{}: quit, {}: menu", quit_key, menu_key))
                    .block(Block::new().borders(Borders::TOP))
            }
        };
        frame.render_widget(footer, layout[1]);
        self.menu.draw(frame, area)?;
        Ok(())
    }
}
