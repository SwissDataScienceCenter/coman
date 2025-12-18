use eyre::{Context, Report};
use tokio::sync::mpsc;
use tuirealm::{
    Application, AttrValue, Attribute, Update,
    ratatui::{
        Frame,
        layout::{Constraint, Direction, Layout, Rect},
        widgets::Clear,
    },
    terminal::{TerminalAdapter, TerminalBridge},
};

use crate::{
    app::{
        ids::Id,
        messages::{
            CscsMsg, DownloadPopupMsg, ErrorPopupMsg, InfoPopupMsg, JobMsg, LoginPopupMsg, MenuMsg, Msg, StatusMsg,
            SystemSelectMsg, View,
        },
        user_events::{CscsEvent, StatusEvent, UserEvent},
    },
    components::{
        context_menu::ContextMenu, download_popup::DownloadTargetInput, error_popup::ErrorPopup, info_popup::InfoPopup,
        login_popup::LoginPopup, system_select_popup::SystemSelectPopup, workload_details::WorkloadDetails,
        workload_list::WorkloadList, workload_log::WorkloadLog,
    },
    cscs::{
        handlers::{cscs_login, cscs_system_set},
        ports::{BackgroundTask, JobLogAction},
    },
    trace_dbg,
    util::ui::{draw_area_in_absolute, draw_area_in_absolute_fixed_height},
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

    /// Determines what view is display
    pub current_view: View,
    /// Used to draw to terminal
    pub terminal: TerminalBridge<T>,

    ///Used to allow sending errors from tokio::spawn async jobs
    pub error_tx: mpsc::Sender<String>,

    /// Triggers async request to select current system
    pub select_system_tx: mpsc::Sender<()>,

    /// Triggers watching job logs
    /// sending None stops watching
    pub job_log_tx: mpsc::Sender<JobLogAction>,

    /// Allows creating user events based on messages
    pub user_event_tx: mpsc::Sender<UserEvent>,

    /// Allows interacting with the file Api
    pub background_task_tx: mpsc::Sender<BackgroundTask>,
}

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn new(
        app: Application<Id, Msg, UserEvent>,
        bridge: TerminalBridge<T>,
        error_tx: mpsc::Sender<String>,
        select_system_tx: mpsc::Sender<()>,
        job_log_tx: mpsc::Sender<JobLogAction>,
        user_event_tx: mpsc::Sender<UserEvent>,
        background_task_tx: mpsc::Sender<BackgroundTask>,
    ) -> Self {
        Self {
            app,
            quit: false,
            redraw: true,
            terminal: bridge,
            current_view: View::default(),
            error_tx,
            select_system_tx,
            job_log_tx,
            user_event_tx,
            background_task_tx,
        }
    }

    pub fn view(&mut self) {
        let terminal = &mut self.terminal;
        let app = &mut self.app;
        let current_view = &self.current_view;
        assert!(
            terminal
                .draw(|f| {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(
                            [
                                Constraint::Max(3),  //Statusbar
                                Constraint::Min(10), //content
                                Constraint::Max(1),  //Toolbar
                            ]
                            .as_ref(),
                        )
                        .split(f.area());
                    app.view(&Id::StatusBar, f, chunks[0]);
                    match current_view {
                        View::Workloads => Self::view_workloads(app, f, chunks[1]),
                        View::Files => Self::view_files(app, f, chunks[1]),
                    }
                    app.view(&Id::Toolbar, f, chunks[2]);

                    if app.mounted(&Id::Menu) {
                        let popup = draw_area_in_absolute(f.area(), 10);
                        f.render_widget(Clear, popup);
                        app.view(&Id::Menu, f, popup);
                    } else if app.mounted(&Id::ErrorPopup) {
                        let popup = draw_area_in_absolute(f.area(), 10);
                        f.render_widget(Clear, popup);
                        app.view(&Id::ErrorPopup, f, popup);
                    } else if app.mounted(&Id::InfoPopup) {
                        let popup = draw_area_in_absolute(f.area(), 10);
                        f.render_widget(Clear, popup);
                        app.view(&Id::InfoPopup, f, popup);
                    } else if app.mounted(&Id::LoginPopup) {
                        let popup = draw_area_in_absolute(f.area(), 10);
                        f.render_widget(Clear, popup);
                        app.view(&Id::LoginPopup, f, popup);
                    } else if app.mounted(&Id::SystemSelectPopup) {
                        let popup = draw_area_in_absolute(f.area(), 10);
                        f.render_widget(Clear, popup);
                        app.view(&Id::SystemSelectPopup, f, popup);
                    } else if app.mounted(&Id::DownloadPopup) {
                        let popup = draw_area_in_absolute_fixed_height(f.area(), 10, 3);
                        f.render_widget(Clear, popup);
                        app.view(&Id::DownloadPopup, f, popup);
                    }
                })
                .is_ok()
        );
    }

    fn view_workloads(app: &mut Application<Id, Msg, UserEvent>, frame: &mut Frame, area: Rect) {
        app.view(&Id::WorkloadList, frame, area);
        app.view(&Id::WorkloadLogs, frame, area);
        app.view(&Id::WorkloadDetails, frame, area);
    }
    fn view_files(app: &mut Application<Id, Msg, UserEvent>, frame: &mut Frame, area: Rect) {
        if app.mounted(&Id::FileView) {
            app.view(&Id::FileView, frame, area);
        }
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
    fn handle_system_select_popup_msg(&mut self, msg: SystemSelectMsg) -> Option<Msg> {
        match msg {
            SystemSelectMsg::Opened(systems) => {
                assert!(
                    self.app
                        .mount(Id::SystemSelectPopup, Box::new(SystemSelectPopup::new(systems)), vec![])
                        .is_ok()
                );
                assert!(self.app.active(&Id::SystemSelectPopup).is_ok());
                None
            }
            SystemSelectMsg::Closed => {
                assert!(self.app.umount(&Id::SystemSelectPopup).is_ok());
                None
            }
            SystemSelectMsg::SystemSelected(system) => {
                assert!(self.app.umount(&Id::SystemSelectPopup).is_ok());
                Some(Msg::Cscs(CscsMsg::SystemSelected(system)))
            }
        }
    }
    fn handle_error_popup_msg(&mut self, msg: ErrorPopupMsg) -> Option<Msg> {
        match msg {
            ErrorPopupMsg::Opened(error_msg) => {
                if self.app.mounted(&Id::ErrorPopup) {
                    assert!(self.app.umount(&Id::ErrorPopup).is_ok());
                }
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
    fn handle_download_popup_msg(&mut self, msg: DownloadPopupMsg) -> Option<Msg> {
        match msg {
            DownloadPopupMsg::Opened(remote_path) => {
                if self.app.mounted(&Id::DownloadPopup) {
                    assert!(self.app.umount(&Id::DownloadPopup).is_ok());
                }
                assert!(
                    self.app
                        .mount(
                            Id::DownloadPopup,
                            Box::new(DownloadTargetInput::new(remote_path)),
                            vec![]
                        )
                        .is_ok()
                );
                assert!(self.app.active(&Id::DownloadPopup).is_ok());
                None
            }
            DownloadPopupMsg::PathSet(remote, local) => {
                assert!(self.app.umount(&Id::DownloadPopup).is_ok());
                let file_tx = self.background_task_tx.clone();
                tokio::spawn(async move {
                    file_tx.send(BackgroundTask::DownloadFile(remote, local)).await.unwrap();
                });
                None
            }
            DownloadPopupMsg::Closed => {
                assert!(self.app.umount(&Id::DownloadPopup).is_ok());
                None
            }
        }
    }
    fn handle_menu_msg(&mut self, msg: MenuMsg) -> Option<Msg> {
        match msg {
            MenuMsg::Opened => {
                assert!(
                    self.app
                        .mount(Id::Menu, Box::new(ContextMenu::new(self.current_view)), vec![])
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
            MenuMsg::CscsSwitchSystem => {
                assert!(self.app.umount(&Id::Menu).is_ok());
                Some(Msg::Cscs(CscsMsg::SelectSystem))
            }
            MenuMsg::Event(event) => {
                assert!(self.app.umount(&Id::Menu).is_ok());
                Some(Msg::CreateEvent(event))
            }
        }
    }
    fn handle_job_msg(&mut self, msg: JobMsg) -> Option<Msg> {
        match msg {
            JobMsg::Log(jobid) => {
                if self.app.mounted(&Id::WorkloadList) {
                    assert!(
                        self.app
                            .attr(&Id::WorkloadList, Attribute::Display, AttrValue::Flag(false))
                            .is_ok()
                    );
                }
                if self.app.mounted(&Id::WorkloadDetails) {
                    assert!(self.app.umount(&Id::WorkloadDetails).is_ok());
                }
                if !self.app.mounted(&Id::WorkloadLogs) {
                    assert!(
                        self.app
                            .mount(Id::WorkloadLogs, Box::new(WorkloadLog::new()), vec![])
                            .is_ok()
                    );
                }
                assert!(self.app.active(&Id::WorkloadLogs).is_ok());
                let job_log_tx = self.job_log_tx.clone();
                tokio::spawn(async move {
                    job_log_tx.send(JobLogAction::Job(jobid)).await.unwrap();
                });
                None
            }
            JobMsg::GetDetails(jobid) => {
                let background_tx = self.background_task_tx.clone();
                let event_tx = self.user_event_tx.clone();
                tokio::spawn(async move {
                    background_tx.send(BackgroundTask::GetJobDetails(jobid)).await.unwrap();
                    event_tx
                        .send(UserEvent::Status(StatusEvent::Info(
                            "getting job details...".to_owned(),
                        )))
                        .await
                        .unwrap();
                });
                None
            }
            JobMsg::Details(jobdetail) => {
                if self.app.mounted(&Id::WorkloadList) {
                    assert!(
                        self.app
                            .attr(&Id::WorkloadList, Attribute::Display, AttrValue::Flag(false))
                            .is_ok()
                    );
                }
                if !self.app.mounted(&Id::WorkloadDetails) {
                    assert!(
                        self.app
                            .mount(Id::WorkloadDetails, Box::new(WorkloadDetails::new(jobdetail)), vec![])
                            .is_ok()
                    );
                }
                assert!(self.app.active(&Id::WorkloadDetails).is_ok());
                None
            }
            JobMsg::Switch => {
                let job_log_tx = self.job_log_tx.clone();
                tokio::spawn(async move {
                    job_log_tx.send(JobLogAction::SwitchLog).await.unwrap();
                });
                None
            }
            JobMsg::Close => {
                if self.app.mounted(&Id::WorkloadLogs) {
                    assert!(self.app.umount(&Id::WorkloadLogs).is_ok());
                }
                if self.app.mounted(&Id::WorkloadDetails) {
                    assert!(self.app.umount(&Id::WorkloadDetails).is_ok());
                }
                if !self.app.mounted(&Id::WorkloadList) {
                    assert!(
                        self.app
                            .mount(Id::WorkloadList, Box::new(WorkloadList::default()), vec![])
                            .is_ok()
                    );
                }
                assert!(
                    self.app
                        .attr(&Id::WorkloadList, Attribute::Display, AttrValue::Flag(true))
                        .is_ok()
                );
                assert!(self.app.active(&Id::WorkloadList).is_ok());
                let job_log_tx = self.job_log_tx.clone();
                tokio::spawn(async move {
                    // stopp polling for logs
                    job_log_tx.send(JobLogAction::Stop).await.unwrap();
                });
                None
            }
        }
    }
    fn change_view(&mut self, view: View) {
        self.current_view = view;
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
            let msg = match msg {
                Msg::None => msg,
                _ => trace_dbg!(msg),
            };
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
                Msg::DownloadPopup(popup_msg) => self.handle_download_popup_msg(popup_msg),
                Msg::Cscs(CscsMsg::Login(client_id, client_secret)) => {
                    let event_tx = self.user_event_tx.clone();
                    let error_tx = self.error_tx.clone();
                    tokio::spawn(async move {
                        match cscs_login(client_id, client_secret).await {
                            Ok(_) => event_tx.send(UserEvent::Cscs(CscsEvent::LoggedIn)).await.unwrap(),
                            Err(e) => error_tx
                                .send(format!(
                                    "{:?}",
                                    Err::<(), Report>(e).wrap_err("Login failed with supplied credentials")
                                ))
                                .await
                                .unwrap(),
                        };
                    });
                    None
                }
                Msg::Cscs(CscsMsg::SelectSystem) => {
                    let system_select_tx = self.select_system_tx.clone();
                    tokio::spawn(async move {
                        system_select_tx.send(()).await.unwrap();
                    });
                    None
                }
                Msg::Cscs(CscsMsg::SystemSelected(system)) => {
                    let event_tx = self.user_event_tx.clone();
                    let error_tx = self.error_tx.clone();
                    tokio::spawn(async move {
                        match cscs_system_set(system.clone(), true).await {
                            Ok(_) => {}
                            Err(e) => error_tx
                                .send(format!(
                                    "{:?}",
                                    Err::<(), Report>(e).wrap_err("failed to set current system")
                                ))
                                .await
                                .unwrap(),
                        };
                        event_tx
                            .send(UserEvent::Cscs(CscsEvent::SystemSelected(system)))
                            .await
                            .unwrap();
                    });
                    None
                }
                Msg::LoginPopup(msg) => self.handle_login_popup_msg(msg),
                Msg::SystemSelectPopup(msg) => self.handle_system_select_popup_msg(msg),
                Msg::Job(msg) => self.handle_job_msg(msg),
                Msg::ChangeView(view) => {
                    self.change_view(view);
                    let event_tx = self.user_event_tx.clone();
                    tokio::spawn(async move {
                        event_tx.send(UserEvent::SwitchedToView(view)).await.unwrap();
                    });
                    None
                }
                Msg::CreateEvent(event) => {
                    let event_tx = self.user_event_tx.clone();
                    tokio::spawn(async move {
                        event_tx.send(event).await.unwrap();
                    });
                    None
                }
                Msg::Status(status) => {
                    let event_tx = self.user_event_tx.clone();
                    let event = match status {
                        StatusMsg::Progress(msg, progress) => StatusEvent::Progress(msg, progress),
                        StatusMsg::Info(msg) => StatusEvent::Info(msg),
                        StatusMsg::Warning(msg) => StatusEvent::Warning(msg),
                    };
                    tokio::spawn(async move {
                        event_tx.send(UserEvent::Status(event)).await.unwrap();
                    });
                    None
                }
                Msg::None => None,
            }
        } else {
            None
        }
    }
}
