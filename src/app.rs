use ratatui::crossterm::event::Event;
use ratatui::prelude::*;
use ratatui::widgets::{ListItem, ListState};

use crate::ui::ui_runai::{WorkloadList, WorkloadStatus};

pub enum ResourceType {
    RunAI,
    CSCS,
}

pub enum CurrentScreen {
    Main,
    Exiting,
}

pub struct App<'a> {
    pub current_screen: CurrentScreen,
    pub resource_type: ResourceType,
    pub inference_list: WorkloadList<'a>,
    pub training_list: WorkloadList<'a>,
}

impl<'a> App<'a> {
    pub fn new() -> App<'a> {
        let training_list = WorkloadList::new("Training").items(vec![
            (WorkloadStatus::Running, "Climate Model training"),
            (WorkloadStatus::Failed, "Markov test"),
            (WorkloadStatus::Stopped, "LLM finetuning job 1"),
        ]);
        let inference_list = WorkloadList::new("Inference").items(vec![
            (WorkloadStatus::Failed, "LLM Hosting"),
            (WorkloadStatus::Stopped, "Climate Model Showcase"),
            (WorkloadStatus::Running, "PDF Extraction"),
        ]);
        App {
            current_screen: CurrentScreen::Main,
            resource_type: ResourceType::RunAI,
            training_list,
            inference_list,
        }
    }
    pub fn handle_event(&mut self, event: Event) {
        self.training_list.handle_event(event.clone());
        self.inference_list.handle_event(event.clone());
    }
}
