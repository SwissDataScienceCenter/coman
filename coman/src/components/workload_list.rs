use tui_realm_stdlib::List;
use tuirealm::{
    Component, Event, MockComponent,
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent},
    props::{Alignment, BorderType, Borders, Color},
};

use crate::app::{messages::Msg, user_events::UserEvent};

#[derive(MockComponent)]
pub(crate) struct WorkloadList {
    component: List,
}

impl Default for WorkloadList {
    fn default() -> Self {
        Self {
            component: List::default()
                .borders(
                    Borders::default()
                        .modifiers(BorderType::Rounded)
                        .color(Color::Yellow),
                )
                .title("Workloads", Alignment::Center)
                .scroll(true)
                .highlighted_color(Color::LightYellow)
                .highlighted_str("-")
                .rewind(true)
                .step(4),
        }
    }
}

impl Component<Msg, UserEvent> for WorkloadList {
    fn on(&mut self, ev: tuirealm::Event<UserEvent>) -> Option<Msg> {
        let _ = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => self.perform(Cmd::Move(Direction::Down)),
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => {
                self.perform(Cmd::Move(Direction::Up))
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                ..
            }) => self.perform(Cmd::Scroll(Direction::Down)),
            Event::Keyboard(KeyEvent {
                code: Key::PageUp, ..
            }) => self.perform(Cmd::Scroll(Direction::Up)),
            Event::Keyboard(KeyEvent {
                code: Key::Home, ..
            }) => self.perform(Cmd::GoTo(Position::Begin)),
            Event::Keyboard(KeyEvent { code: Key::End, .. }) => {
                self.perform(Cmd::GoTo(Position::End))
            }
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}

// use color_eyre::Result;
// use ratatui::{prelude::*, widgets::*};
// use tokio::sync::mpsc::UnboundedSender;

// use super::Component;
// use crate::{action::Action, config::Config, focus_manager::Focus};

// #[derive(Default)]
// pub struct WorkloadList<'a> {
//     command_tx: Option<UnboundedSender<Action>>,
//     config: Config,
//     state: ListState,
//     last_area: Rect,
//     list_items: Vec<ListItem<'a>>,
//     id: String,
//     focus: Focus,
// }

// #[allow(dead_code)]
// impl<'a> WorkloadList<'a> {
//     pub fn new(id: String) -> Self {
//         Self {
//             last_area: Rect::ZERO,
//             id,
//             ..Self::default()
//         }
//     }
//     pub fn select_none(&mut self) {
//         self.state.select(None);
//     }
//     pub fn select_next(&mut self) {
//         self.state.select_next();
//     }
//     pub fn select_previous(&mut self) {
//         self.state.select_previous();
//     }
//     pub fn select_first(&mut self) {
//         self.state.select_first();
//     }
//     pub fn select_last(&mut self) {
//         self.state.select_last();
//     }
// }

// impl<'a> Component for WorkloadList<'a> {
//     fn id(&self) -> String {
//         self.id.clone()
//     }
//     fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
//         self.command_tx = Some(tx);
//         Ok(())
//     }

//     fn register_config_handler(&mut self, config: Config) -> Result<()> {
//         self.config = config;
//         Ok(())
//     }

//     fn update(&mut self, action: Action) -> Result<Option<Action>> {
//         match action {
//             Action::Tick => {
//                 // add any logic here that should run on every tick
//             }
//             Action::Render => {
//                 // add any logic here that should run on every render
//             }
//             Action::FocusChanged(component_id, focus) => {
//                 if component_id == self.id {
//                     self.focus = focus;
//                 }
//             }
//             _ => {}
//         }
//         Ok(None)
//     }

//     fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
//         self.last_area = area;
//         frame.render_widget(Paragraph::new("hello world"), area);
//         Ok(())
//     }
// }
