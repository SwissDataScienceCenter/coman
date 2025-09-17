use ratatui::prelude::*;
use ratatui::widgets::{List, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::Text,
    widgets::{Block, Borders, Paragraph},
};

use crate::app::{App, CurrentScreen, ResourceType};

// ANCHOR: method_sig
pub fn ui(frame: &mut Frame, app: &App) {
    let outer_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let inner_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(outer_layout[1]);
    let title_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default());

    let title = Paragraph::new(Text::styled(
        "Compute Manager",
        Style::default().fg(Color::Green),
    ))
    .block(title_block);
    frame.render_widget(title, outer_layout[0]);

    let mut workloads = vec!["my training job", "climate model"];
    let list = List::new(workloads)
        .style(Style::new().white())
        .highlight_style(Style::new().green())
        .highlight_symbol("> ");
    let mut workload_state = ListState::default();
    frame.render_stateful_widget(list, inner_layout[0], &mut workload_state);

    let current_navigation_text = vec![
        // The first half of the text
        match app.current_screen {
            CurrentScreen::Main => Span::styled(
                format!(
                    "[{}]",
                    match app.resource_type {
                        ResourceType::RunAI => "RunAI",
                        ResourceType::CSCS => "CSCS",
                    }
                ),
                Style::default().fg(Color::Green),
            ),
            CurrentScreen::Exiting => Span::styled("Exiting", Style::default().fg(Color::LightRed)),
        }
        .to_owned(),
        // A white divider bar to separate the two sections
        Span::styled(" | ", Style::default().fg(Color::White)),
        // The final section of the text, with hints on what the user is editing
        Span::styled(
            "↑←↓→: navigate, x: menu, q: quit",
            Style::default().fg(Color::LightBlue),
        ),
    ];

    let mut logs = vec![
        "2025-09-16 13:36:46.758 UTC [27] LOG:  checkpoint starting: time",
        "2025-09-16 13:36:47.902 UTC [27] LOG:  checkpoint complete: wrote 12 buffers (0.1%); 0 WAL file(s) added, 0 removed, 0 recycled; write=1.103 s, sync=0.008 s, total=1.144 s; sync files=9, longest=0.003 s, average=0.001 s; distance=51 kB, estimate=51 kB; lsn=0/37FD138, redo lsn=0/37FD100",
        "2025-09-16 13:41:47.002 UTC [27] LOG:  checkpoint starting: time",
        "2025-09-16 13:41:47.718 UTC [27] LOG:  checkpoint complete: wrote 8 buffers (0.0%); 0 WAL file(s) added, 0 removed, 0 recycled; write=0.703 s, sync=0.008 s, total=0.716 s; sync files=7, longest=0.002 s, average=0.002 s; distance=21 kB, estimate=48 kB; lsn=0/38025F0, redo lsn=0/38025B8",
        "2025-09-16 13:46:47.783 UTC [27] LOG:  checkpoint starting: time",
        "2025-09-16 13:46:48.502 UTC [27] LOG:  checkpoint complete: wrote 8 buffers (0.0%); 0 WAL file(s) added, 0 removed, 0 recycled; write=0.704 s, sync=0.008 s, total=0.719 s; sync files=7, longest=0.002 s, average=0.002 s; distance=22 kB, estimate=45 kB; lsn=0/3807FF8, redo lsn=0/3807FC0",
        "2025-09-16 13:51:47.602 UTC [27] LOG:  checkpoint starting: time",
        "2025-09-16 13:51:48.850 UTC [27] LOG:  checkpoint complete: wrote 13 buffers (0.1%); 0 WAL file(s) added, 0 removed, 0 recycled; write=1.205 s, sync=0.010 s, total=1.249 s; sync files=10, longest=0.002 s, average=0.001 s; distance=36 kB, estimate=44 kB; lsn=0/3811238, redo lsn=0/3811200",
        "2025-09-16 13:56:47.950 UTC [27] LOG:  checkpoint starting: time",
        "2025-09-16 13:56:48.771 UTC [27] LOG:  checkpoint complete: wrote 9 buffers (0.1%); 0 WAL file(s) added, 0 removed, 0 recycled; write=0.803 s, sync=0.008 s, total=0.821 s; sync files=8, longest=0.002 s, average=0.001 s; distance=10 kB, estimate=41 kB; lsn=0/3813E10, redo lsn=0/3813DD8",
        "2025-09-16 14:01:47.826 UTC [27] LOG:  checkpoint starting: time",
        "2025-09-16 14:01:48.543 UTC [27] LOG:  checkpoint complete: wrote 8 buffers (0.0%); 0 WAL file(s) added, 0 removed, 0 recycled; write=0.703 s, sync=0.008 s, total=0.717 s; sync files=7, longest=0.002 s, average=0.002 s; distance=12 kB, estimate=38 kB; lsn=0/3816E10, redo lsn=0/3816DD8",
        "2025-09-16 14:06:47.624 UTC [27] LOG:  checkpoint starting: time",
        "2025-09-16 14:06:48.348 UTC [27] LOG:  checkpoint complete: wrote 8 buffers (0.0%); 0 WAL file(s) added, 0 removed, 0 recycled; write=0.703 s, sync=0.015 s, total=0.725 s; sync files=7, longest=0.003 s, average=0.003 s; distance=9 kB, estimate=35 kB; lsn=0/3819228, redo lsn=0/38191F0",
    ];
    logs.extend_from_within(..);
    logs.extend_from_within(..);
    logs.extend_from_within(..);
    logs.extend_from_within(..);
    let mut log_state = ListState::default();
    let log_list = List::new(logs)
        .style(Style::default().fg(Color::White))
        // .scroll((app.content_scroll, 0))
        .block(Block::default().borders(Borders::LEFT));

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    let mut scrollbar_state = ScrollbarState::new(log_list.len()).position(log_state.offset());
    let area = frame.area();
    frame.render_stateful_widget(log_list, inner_layout[1], &mut log_state);
    frame.render_stateful_widget(
        scrollbar,
        area.inner(Margin {
            horizontal: 0,
            vertical: 1,
        }),
        &mut scrollbar_state,
    );
    let mode_footer = Paragraph::new(Line::from(current_navigation_text))
        .block(Block::default().borders(Borders::TOP));

    frame.render_widget(mode_footer, outer_layout[2]);
}
