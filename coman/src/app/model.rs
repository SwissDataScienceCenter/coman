use tuirealm::{
    Application, Update,
    ratatui::layout::{Constraint, Direction, Layout},
    ratatui::widgets::Clear,
    terminal::{TerminalAdapter, TerminalBridge},
};

use crate::{
    app::{
        ids::Id,
        messages::{ErrorPopupMsg, MenuMsg, Msg},
        user_events::UserEvent,
    },
    components::{error_popup::ErrorPopup, workload_menu::WorkloadMenu},
    util::ui::draw_area_in_absolute,
};

pub struct Model<T>
where
    T: TerminalAdapter,
{
    /// Application
    pub app: Application<Id, Msg, UserEvent>,
    /// Indicates that the application must quit
    pub quit: bool,
    /// Tells whether to redraw interface
    pub redraw: bool,
    /// Used to draw to terminal
    pub terminal: TerminalBridge<T>,
}

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn new(app: Application<Id, Msg, UserEvent>, adapter: T) -> Self {
        Self {
            app,
            quit: false,
            redraw: true,
            terminal: TerminalBridge::init(adapter).expect("Cannot initialize terminal"),
        }
    }

    pub fn view(&mut self) {
        assert!(
            self.terminal
                .draw(|f| {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(1)
                        .constraints(
                            [
                                Constraint::Min(10), //WorkloadList
                                Constraint::Max(1),  //Toolbar
                            ]
                            .as_ref(),
                        )
                        .split(f.area());
                    self.app.view(&Id::WorkloadList, f, chunks[0]);
                    self.app.view(&Id::Toolbar, f, chunks[1]);

                    if self.app.mounted(&Id::Menu) {
                        let popup = draw_area_in_absolute(f.area(), 30, 20);
                        f.render_widget(Clear, popup);
                        self.app.view(&Id::Menu, f, popup);
                    }
                })
                .is_ok()
        );
    }
    fn handle_error_popup_msg(&mut self, msg: ErrorPopupMsg) -> Option<Msg> {
        match msg {
            ErrorPopupMsg::Opened(error_msg) => {
                assert!(
                    self.app
                        .mount(Id::ErrorPopup, Box::new(ErrorPopup::default()), vec![])
                        .is_ok()
                );
                assert!(self.app.active(&Id::ErrorPopup).is_ok());
                None
            }
            ErrorPopupMsg::Closed => {
                assert!(self.app.umount(&Id::ErrorPopup).is_ok());
                None
            }
        }
    }
    fn handle_menu_msg(&mut self, msg: MenuMsg) -> Option<Msg> {
        match msg {
            MenuMsg::Opened => {
                assert!(
                    self.app
                        .mount(Id::Menu, Box::new(WorkloadMenu::default()), vec![])
                        .is_ok()
                );
                assert!(self.app.active(&Id::Menu).is_ok());
                None
            }
            MenuMsg::Closed => {
                assert!(self.app.umount(&Id::Menu).is_ok());
                None
            }
            MenuMsg::CSCSLogin => {
                assert!(self.app.umount(&Id::Menu).is_ok());
                Some(Msg::CSCSLogin)
            }
        }
    }
}

// Let's implement Update for model

impl<T> Update<Msg> for Model<T>
where
    T: TerminalAdapter,
{
    fn update(&mut self, msg: Option<Msg>) -> Option<Msg> {
        if let Some(msg) = msg {
            // Set redraw
            self.redraw = true;
            // Match message
            match msg {
                Msg::AppClose => {
                    self.quit = true; // Terminate
                    None
                }
                Msg::Error(error_msg) => Some(Msg::ErrorPopup(ErrorPopupMsg::Opened(error_msg))),
                Msg::Menu(menu_msg) => self.handle_menu_msg(menu_msg),
                Msg::ErrorPopup(popup_msg) => self.handle_error_popup_msg(popup_msg),
                Msg::CSCSLogin => None,
                Msg::CSCSToken(_, _) => None,

                Msg::None => None,
            }
        } else {
            None
        }
    }
}
