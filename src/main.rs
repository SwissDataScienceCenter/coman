use std::{
    error::Error,
    io,
    panic::{set_hook, take_hook},
};

use ratatui::{
    Terminal,
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    prelude::{Backend, CrosstermBackend},
};

mod app;
mod ui;
use crate::{
    app::{App, CurrentScreen},
    ui::ui,
};
fn main() -> Result<(), Box<dyn Error>> {
    // create app and run it
    cli_log::init_cli_log!();
    init_panic_hook();
    let mut terminal = init_tui()?;
    let mut app = App::new();
    let _res = run_app(&mut terminal, &mut app);
    restore_tui()?;

    Ok(())
}
fn init_panic_hook() {
    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        let _ = restore_tui();
        original_hook(panic_info);
    }));
}

fn init_tui() -> io::Result<Terminal<impl Backend>> {
    enable_raw_mode()?;
    let mut stderr = io::stderr();
    execute!(stderr, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stderr);
    Terminal::new(backend)
}
fn restore_tui() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;
        let event = event::read()?;
        if let Event::Key(key) = event {
            if key.kind == event::KeyEventKind::Release {
                // Skip events that are not KeyEventKind::Press
                continue;
            }
            if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(());
            }
        }
        app.handle_event(event);
    }
}
