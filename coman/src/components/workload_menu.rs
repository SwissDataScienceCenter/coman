use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    app::{Mode, SubMode},
    components::Component,
    config::Config,
    focus_manager::Focus,
};
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
    id: String,
    focus: Focus,
}

#[allow(dead_code)]
impl<'a> WorkloadListMenu<'a> {
    pub fn new(id: String) -> Self {
        Self {
            last_area: Rect::ZERO,
            items: vec![
                MenuItem::new("Login to CSCS", Action::CSCSLogin),
                MenuItem::new("todo", Action::Escape),
            ],
            id,
            ..Self::default()
        }
    }
    fn show(&self) -> bool {
        match self.sub_mode {
            SubMode::Main => false,
            SubMode::Menu => true,
        }
    }
    fn is_focused(&self) -> bool {
        match self.focus {
            Focus::Active | Focus::Permanent | Focus::Exclusive => true,
            _ => false,
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
    fn id(&self) -> String {
        self.id.clone()
    }
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
                    return Ok(Some(Action::RequestFocus(self.id.clone(), Focus::Active)));
                }
            }
            Action::Up => {
                if self.show() && self.is_focused() {
                    self.select_previous();
                }
            }
            Action::Down => {
                if self.show() && self.is_focused() {
                    self.select_next();
                }
            }
            Action::Enter => {
                if !self.is_focused() {
                    return Ok(None);
                }
                if let Some(index) = self.state.selected() {
                    self.command_tx.as_mut().map(|tx| tx.send(Action::Menu));
                    self.command_tx
                        .clone()
                        .as_mut()
                        .map(|tx| tx.send(Action::ReleaseFocus(self.id())));
                    self.select_none();
                    return Ok(Some(self.items[index].action.clone()));
                }
            }
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
