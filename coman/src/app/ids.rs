#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum Id {
    StatusBar,
    Toolbar,
    WorkloadList,
    WorkloadLogs,
    GlobalListener,
    Menu,
    InfoPopup,
    ErrorPopup,
    LoginPopup,
    DownloadPopup,
    SystemSelectPopup,
    FileView,
}
