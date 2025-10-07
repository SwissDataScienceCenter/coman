use color_eyre::{Result, owo_colors::OwoColorize};
use crossterm::event::KeyEvent;
use ratatui::prelude::Rect;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::{
    action::{Action, ErrorDetail},
    components::{
        Component, error_popup::ErrorPopup, footer::Footer, workload_list::WorkloadList,
        workload_menu::WorkloadListMenu,
    },
    config::Config,
    focus_manager::{Focus, FocusManager},
    trace_dbg,
    tui::{Event, Tui},
    util,
};

pub struct App {
    config: Config,
    tick_rate: f64,
    frame_rate: f64,
    components: Vec<Box<dyn Component>>,
    focus_manager: FocusManager,
    should_quit: bool,
    should_suspend: bool,
    mode: Mode,
    sub_mode: SubMode,
    last_tick_key_events: Vec<KeyEvent>,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
}
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubMode {
    #[default]
    Main,
    Menu,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Main,
}

impl App {
    pub fn new(tick_rate: f64, frame_rate: f64) -> Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let workload_list_id = "WorkloadList".to_string();
        let workload_list = WorkloadList::new(workload_list_id.clone());
        let footer_id = "Footer".to_string();
        let footer = Footer::new(footer_id.clone());
        let focus_manager = FocusManager::new(workload_list_id);
        action_tx.send(Action::RequestFocus(footer_id.clone(), Focus::Permanent))?;
        Ok(Self {
            tick_rate,
            frame_rate,
            components: vec![
                Box::new(workload_list),
                Box::new(footer),
                Box::new(ErrorPopup::new("ErrorPopup".to_string())),
            ],
            focus_manager: focus_manager,
            should_quit: false,
            should_suspend: false,
            config: Config::new()?,
            mode: Mode::Main,
            sub_mode: SubMode::Main,
            last_tick_key_events: Vec::new(),
            action_tx,
            action_rx,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?
            .mouse(true)
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);
        tui.enter()?;

        for component in self.components.iter_mut() {
            component.register_action_handler(self.action_tx.clone())?;
        }
        for component in self.components.iter_mut() {
            component.register_config_handler(self.config.clone())?;
        }
        for component in self.components.iter_mut() {
            component.init(tui.size()?)?;
        }

        let action_tx = self.action_tx.clone();
        loop {
            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui)?;
            if self.should_suspend {
                tui.suspend()?;
                action_tx.send(Action::Resume)?;
                action_tx.send(Action::ClearScreen)?;
                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    async fn handle_events(&mut self, tui: &mut Tui) -> Result<()> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };
        let action_tx = self.action_tx.clone();
        match event {
            Event::Quit => action_tx.send(Action::Quit)?,
            Event::Tick => action_tx.send(Action::Tick)?,
            Event::Render => action_tx.send(Action::Render)?,
            Event::RemoteRefresh => action_tx.send(Action::RemoteRefresh)?,
            Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
            Event::Key(key) => self.handle_key_event(key)?,
            _ => {}
        }

        for component in self.components.iter_mut() {
            if let Some(action) = component.handle_events(Some(event.clone()))? {
                action_tx.send(action)?;
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        let action_tx = self.action_tx.clone();
        let Some(keymap) = self.config.keybindings.get(&self.mode) else {
            return Ok(());
        };
        match keymap.get(&vec![key]) {
            Some(action) => {
                info!("Got action: {action:?}");
                action_tx.send(action.clone())?;
            }
            _ => {
                // If the key was not handled as a single key action,
                // then consider it for multi-key combinations.
                self.last_tick_key_events.push(key);

                // Check for multi-key combinations
                if let Some(action) = keymap.get(&self.last_tick_key_events) {
                    info!("Got action: {action:?}");
                    action_tx.send(action.clone())?;
                }
            }
        }
        Ok(())
    }

    fn handle_actions(&mut self, tui: &mut Tui) -> Result<()> {
        while let Ok(action) = self.action_rx.try_recv() {
            match action.clone() {
                Action::Tick => {
                    self.last_tick_key_events.drain(..);
                }
                Action::Quit => self.should_quit = true,
                Action::Suspend => self.should_suspend = true,
                Action::Resume => self.should_suspend = false,
                Action::ClearScreen => tui.terminal.clear()?,
                Action::Resize(w, h) => self.handle_resize(tui, w, h)?,
                Action::Render => self.render(tui)?,
                Action::Mode(mode) => self.mode = mode,
                Action::SubMode(sub_mode) => self.sub_mode = sub_mode,
                Action::Menu => match self.sub_mode {
                    SubMode::Main => {
                        self.action_tx.send(Action::SubMode(SubMode::Menu))?;
                    }
                    SubMode::Menu => {
                        self.action_tx.send(Action::SubMode(SubMode::Main))?;
                    }
                },
                Action::Escape => {
                    if self.sub_mode == SubMode::Menu {
                        self.action_tx.send(Action::SubMode(SubMode::Main))?;
                    }
                }
                Action::CSCSLogin => {
                    let action_tx = self.action_tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = util::cscs_login(action_tx.clone()).await {
                            action_tx
                                .send(Action::Error(crate::action::ErrorDetail::new(
                                    "Couldn't log in to CSCS",
                                    e,
                                )))
                                .unwrap();
                        }
                    });
                }
                Action::CSCSToken(ref access_token, ref refresh_token) => {
                    let access_entry = keyring::Entry::new("coman", "cscs_access_token")?;
                    access_entry.set_password(access_token.as_str())?;
                    if let Some(r) = refresh_token.clone() {
                        let refresh_entry = keyring::Entry::new("coman", "cscs_refresh_token")?;
                        refresh_entry.set_password(r.as_str())?;
                    }
                    self.action_tx.send(Action::RemoteRefresh)?;
                }
                Action::Error(err) => {
                    tracing::event!(target: module_path!(),  tracing::Level::ERROR, err.full);
                }
                _ => {}
            }
            if action != Action::Render && action != Action::Tick {
                trace_dbg!(action.clone());
            }
            if let Some(follow_action) = self.focus_manager.update(action.clone())? {
                self.action_tx.send(follow_action)?;
            }

            for component in self.components.iter_mut() {
                if self.focus_manager.should_receive_event(component.id()) {
                    if let Some(action) = component.update(action.clone())? {
                        self.action_tx.send(action)?
                    };
                }
            }
        }
        Ok(())
    }

    fn handle_resize(&mut self, tui: &mut Tui, w: u16, h: u16) -> Result<()> {
        tui.resize(Rect::new(0, 0, w, h))?;
        self.render(tui)?;
        Ok(())
    }

    fn render(&mut self, tui: &mut Tui) -> Result<()> {
        tui.draw(|frame| {
            for component in self.components.iter_mut() {
                if let Err(err) = component.draw(frame, frame.area()) {
                    let _ = self
                        .action_tx
                        .send(Action::Error(ErrorDetail::new("Failed to draw", err)));
                }
            }
        })?;
        Ok(())
    }
}
