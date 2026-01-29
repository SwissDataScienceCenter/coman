#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum Id {
    StatusBar,
    Toolbar,
    WorkloadList,
    WorkloadLogs,
    WorkloadDetails,
    WorkloadResourceUsage,
    GlobalListener,
    Menu,
    InfoPopup,
    ErrorPopup,
    LoginPopup,
    DownloadPopup,
    SystemSelectPopup,
    FileView,
}
