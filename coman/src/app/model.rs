use eyre::{Context, Report};
use tuirealm::{
    Application, Update,
    ratatui::layout::{Constraint, Direction, Layout},
    ratatui::widgets::Clear,
    terminal::{TerminalAdapter, TerminalBridge},
};

use crate::{
    app::{
        ids::Id,
        messages::{CscsMsg, ErrorPopupMsg, InfoPopupMsg, LoginPopupMsg, MenuMsg, Msg},
        user_events::UserEvent,
    },
    components::{
        error_popup::ErrorPopup, info_popup::InfoPopup, login_popup::LoginPopup,
        workload_menu::WorkloadMenu,
    },
    cscs::cli::cscs_login,
    trace_dbg,
    util::ui::draw_area_in_absolute,
};
use tokio::sync::mpsc;

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

    ///Used to allow sending errors from tokio::spawn async jobs
    pub error_tx: mpsc::Sender<String>,
}

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn new(
        app: Application<Id, Msg, UserEvent>,
        bridge: TerminalBridge<T>,
        error_tx: mpsc::Sender<String>,
    ) -> Self {
        Self {
            app,
            quit: false,
            redraw: true,
            terminal: bridge,
            error_tx,
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
                        let popup = draw_area_in_absolute(f.area(), 10);
                        f.render_widget(Clear, popup);
                        self.app.view(&Id::Menu, f, popup);
                    } else if self.app.mounted(&Id::ErrorPopup) {
                        let popup = draw_area_in_absolute(f.area(), 10);
                        f.render_widget(Clear, popup);
                        self.app.view(&Id::ErrorPopup, f, popup);
                    } else if self.app.mounted(&Id::InfoPopup) {
                        let popup = draw_area_in_absolute(f.area(), 10);
                        f.render_widget(Clear, popup);
                        self.app.view(&Id::InfoPopup, f, popup);
                    } else if self.app.mounted(&Id::LoginPopup) {
                        let popup = draw_area_in_absolute(f.area(), 10);
                        f.render_widget(Clear, popup);
                        self.app.view(&Id::LoginPopup, f, popup);
                    }
                })
                .is_ok()
        );
    }
    fn handle_login_popup_msg(&mut self, msg: LoginPopupMsg) -> Option<Msg> {
        match msg {
            LoginPopupMsg::Opened => {
                assert!(
                    self.app
                        .mount(Id::LoginPopup, Box::new(LoginPopup::new()), vec![])
                        .is_ok()
                );
                assert!(self.app.active(&Id::LoginPopup).is_ok());
                None
            }
            LoginPopupMsg::Closed => {
                assert!(self.app.umount(&Id::LoginPopup).is_ok());
                None
            }
            LoginPopupMsg::LoginDone(client_id, client_secret) => {
                assert!(self.app.umount(&Id::LoginPopup).is_ok());
                Some(Msg::Cscs(CscsMsg::Login(client_id, client_secret)))
            }
        }
    }
    fn handle_error_popup_msg(&mut self, msg: ErrorPopupMsg) -> Option<Msg> {
        match msg {
            ErrorPopupMsg::Opened(error_msg) => {
                assert!(
                    self.app
                        .mount(Id::ErrorPopup, Box::new(ErrorPopup::new(error_msg)), vec![])
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
    fn handle_info_popup_msg(&mut self, msg: InfoPopupMsg) -> Option<Msg> {
        match msg {
            InfoPopupMsg::Opened(info_msg) => {
                if self.app.mounted(&Id::InfoPopup) {
                    // if there is already an info popup, replace it
                    assert!(self.app.umount(&Id::InfoPopup).is_ok());
                }
                assert!(
                    self.app
                        .mount(Id::InfoPopup, Box::new(InfoPopup::new(info_msg)), vec![])
                        .is_ok()
                );
                assert!(self.app.active(&Id::InfoPopup).is_ok());
                None
            }
            InfoPopupMsg::Closed => {
                assert!(self.app.umount(&Id::InfoPopup).is_ok());
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
            MenuMsg::CscsLogin => {
                assert!(self.app.umount(&Id::Menu).is_ok());
                Some(Msg::LoginPopup(LoginPopupMsg::Opened))
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
            // log messages in debug mode
            let msg = trace_dbg!(msg);
            // Set redraw
            self.redraw = true;
            // Match message
            match msg {
                Msg::AppClose => {
                    self.quit = true; // Terminate
                    None
                }
                Msg::Error(error_msg) => Some(Msg::ErrorPopup(ErrorPopupMsg::Opened(error_msg))),
                Msg::Info(info_msg) => Some(Msg::InfoPopup(InfoPopupMsg::Opened(info_msg))),
                Msg::Menu(menu_msg) => self.handle_menu_msg(menu_msg),
                Msg::ErrorPopup(popup_msg) => self.handle_error_popup_msg(popup_msg),
                Msg::InfoPopup(popup_msg) => self.handle_info_popup_msg(popup_msg),
                Msg::Cscs(CscsMsg::Login(client_id, client_secret)) => {
                    let error_tx = self.error_tx.clone();
                    tokio::spawn(async move {
                        match cscs_login(client_id, client_secret).await {
                            Ok(_) => {}
                            Err(e) => error_tx
                                .send(format!(
                                    "{:?}",
                                    Err::<(), Report>(e)
                                        .wrap_err("Login failed with supplied credentials")
                                ))
                                .await
                                .unwrap(),
                        };
                    });
                    None
                }
                Msg::None => None,
                Msg::LoginPopup(msg) => self.handle_login_popup_msg(msg),
            }
        } else {
            None
        }
    }
}
