use color_eyre::Result;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::prelude::*;
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, trace_dbg};

#[derive(Debug, Clone, Copy)]
struct Theme {
    text: Color,
    background: Color,
    highlight: Color,
    shadow: Color,
}
impl Default for Theme {
    fn default() -> Self {
        BLUE
    }
}

const BLUE: Theme = Theme {
    text: Color::Rgb(16, 24, 48),
    background: Color::Rgb(48, 72, 144),
    highlight: Color::Rgb(64, 96, 192),
    shadow: Color::Rgb(32, 48, 96),
};
#[derive(Debug, Clone, Default)]
pub struct Button {
    command_tx: Option<UnboundedSender<Action>>,
    label: String,
    is_pressed: bool,
    theme: Theme,
    last_area: Rect,
    on_click: Option<Action>,
}
impl Button {
    pub fn new(label: String) -> Self {
        Button {
            label,
            last_area: Rect::ZERO,
            ..Default::default()
        }
    }
    fn register_action_handler(
        &mut self,
        tx: UnboundedSender<Action>,
    ) -> color_eyre::eyre::Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }
    pub fn on_click(mut self, on_click: Action) -> Self {
        self.on_click = Some(on_click);
        self
    }
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
    pub fn handle_mouse_event(&mut self, event: MouseEvent) -> Result<Option<Action>> {
        if let MouseEventKind::Up(button) = event.kind
            && button == MouseButton::Left
        {
            let action = if self.is_pressed
                && let Some(action) = self.on_click.clone()
            {
                Some(action)
            } else {
                None
            };
            self.is_pressed = false;
            return Ok(action);
        }
        if !self
            .last_area
            .contains(Position::new(event.column, event.row))
        {
            return Ok(None);
        }
        if let MouseEventKind::Down(MouseButton::Left) = event.kind {
            self.is_pressed = true
        }
        Ok(None)
    }
}

impl Widget for &mut Button {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        self.last_area = area;
        let (background, text, shadow, highlight) = if self.is_pressed {
            (
                self.theme.background,
                self.theme.text,
                self.theme.shadow,
                self.theme.highlight,
            )
        } else {
            (
                self.theme.highlight,
                self.theme.text,
                self.theme.highlight,
                self.theme.shadow,
            )
        };
        buf.set_style(area, Style::new().fg(text).bg(background));
        if area.height > 2 {
            buf.set_string(
                area.x,
                area.y,
                "▔".repeat(area.width as usize),
                Style::new().fg(highlight).bg(background),
            );
        }
        // render bottom line if there's enough space
        if area.height > 1 {
            buf.set_string(
                area.x,
                area.y + area.height - 1,
                "▁".repeat(area.width as usize),
                Style::new().fg(shadow).bg(background),
            );
        }
        let line = Line::raw(self.label.clone());
        // render label centered
        buf.set_line(
            area.x + (area.width.saturating_sub(line.width() as u16)) / 2,
            area.y + (area.height.saturating_sub(1)) / 2,
            &line,
            area.width,
        );
    }
}
