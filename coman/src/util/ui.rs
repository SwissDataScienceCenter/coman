use tuirealm::ratatui::layout::{Constraint, Layout, Rect};

pub fn draw_area_in_absolute(parent: Rect, width: u16, height: u16) -> Rect {
    let new_area = Layout::vertical([
        Constraint::Length((parent.height - height) / 2),
        Constraint::Length(height),
        Constraint::Length((parent.height - height) / 2),
    ])
    .split(parent);
    Layout::horizontal([
        Constraint::Length((parent.width - width) / 2),
        Constraint::Length(width),
        Constraint::Length((parent.width - width) / 2),
    ])
    .split(new_area[1])[1]
}
