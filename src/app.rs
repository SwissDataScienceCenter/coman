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
    pub content_scroll: u16,
}

impl App {
    pub fn new() -> App {
        App {
            current_screen: CurrentScreen::Main,
            resource_type: ResourceType::RunAI,
            content_scroll: 0,
        }
    }
}
