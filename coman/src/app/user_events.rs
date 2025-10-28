#[derive(Debug, Eq, Clone, Copy, PartialEq, PartialOrd, Ord)]
pub struct CSCSWorkloadData {}

#[derive(Debug, Eq, Clone, Copy, PartialOrd, Ord)]
pub enum UserEvent {
    GotCSCSWorkloadData(CSCSWorkloadData),
    None,
}

impl PartialEq for UserEvent {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (
                UserEvent::GotCSCSWorkloadData(_),
                UserEvent::GotCSCSWorkloadData(_)
            )
        )
    }
}
