use ratatui::prelude::*;
use ratatui::widgets::{ListItem, ListState};

pub enum ResourceType {
    RunAI,
    CSCS,
}

pub enum CurrentScreen {
    Main,
    Exiting,
}

pub struct App {
    pub current_screen: CurrentScreen,
    pub resource_type: ResourceType,
    pub inference_list: WorkloadList,
    pub training_list: WorkloadList,
}

pub struct WorkloadList {
    pub items: Vec<WorkloadItem>,
    pub state: ListState,
}

#[derive(Debug)]
pub struct WorkloadItem {
    pub name: String,
    pub status: WorkloadStatus,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorkloadStatus {
    Starting,
    Running,
    Terminating,
    Stopped,
    Failed,
}

impl App {
    pub fn new() -> App {
        App {
            current_screen: CurrentScreen::Main,
            resource_type: ResourceType::RunAI,
            training_list: WorkloadList::from_iter([
                (WorkloadStatus::Running, "Climate Model training"),
                (WorkloadStatus::Failed, "Markov test"),
                (WorkloadStatus::Stopped, "LLM finetuning job 1"),
            ]),
            inference_list: WorkloadList::from_iter([
                (WorkloadStatus::Failed, "LLM Hosting"),
                (WorkloadStatus::Stopped, "Climate Model Showcase"),
                (WorkloadStatus::Running, "PDF Extraction"),
            ]),
        }
    }
}

impl FromIterator<(WorkloadStatus, &'static str)> for WorkloadList {
    fn from_iter<I: IntoIterator<Item = (WorkloadStatus, &'static str)>>(iter: I) -> Self {
        let items = iter
            .into_iter()
            .map(|(status, name)| WorkloadItem::new(status, name))
            .collect();
        let state = ListState::default();
        Self { items, state }
    }
}

impl WorkloadItem {
    fn new(status: WorkloadStatus, name: &str) -> Self {
        Self {
            name: name.to_string(),
            status,
        }
    }
}

impl From<&WorkloadItem> for ListItem<'_> {
    fn from(value: &WorkloadItem) -> Self {
        let line = match value.status {
            WorkloadStatus::Starting => Line::styled(value.name.clone(), Color::Green),
            WorkloadStatus::Running => Line::styled(value.name.clone(), Color::Blue),
            WorkloadStatus::Terminating => Line::styled(value.name.clone(), Color::Yellow),
            WorkloadStatus::Stopped => Line::styled(value.name.clone(), Color::Gray),
            WorkloadStatus::Failed => Line::styled(value.name.clone(), Color::Red),
        };
        ListItem::new(line)
    }
}
