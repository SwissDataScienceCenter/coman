use tuirealm::ratatui::layout::{Constraint, Layout, Rect};

pub fn draw_area_in_absolute(parent: Rect, padding: u16) -> Rect {
    let new_area = Layout::vertical([
        Constraint::Length(padding),
        Constraint::Min(1),
        Constraint::Length(padding),
    ])
    .split(parent);
    Layout::horizontal([
        Constraint::Length(padding),
        Constraint::Min(1),
        Constraint::Length(padding),
    ])
    .split(new_area[1])[1]
}
