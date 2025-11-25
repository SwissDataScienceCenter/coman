use std::time::Duration;
use tokio::sync::mpsc;

use clap::Parser;
use color_eyre::Result;
use keyring::set_global_service_name;
use tokio::runtime::Handle;
use tuirealm::{
    Application, EventListenerCfg, PollStrategy, Sub, SubClause, SubEventClause, Update,
    event::{Key, KeyEvent, KeyModifiers},
    terminal::{CrosstermTerminalAdapter, TerminalBridge},
};

use crate::{
    app::{
        ids::Id,
        messages::Msg,
        model::Model,
        user_events::{CscsEvent, UserEvent},
    },
    cli::{Cli, version},
    components::{global_listener::GlobalListener, toolbar::Toolbar, workload_list::WorkloadList},
    cscs::{
        cli::{
            cli_cscs_job_cancel, cli_cscs_job_detail, cli_cscs_job_list, cli_cscs_job_start,
            cli_cscs_login, cli_cscs_system_list,
        },
        ports::{AsyncDeviceFlowPort, AsyncFetchWorkloadsPort},
    },
    errors::AsyncErrorPort,
};

mod app;
mod cli;
mod components;
mod config;
mod cscs;
mod errors;
mod logging;
mod util;

#[macro_use]
extern crate tuirealm;

#[tokio::main]
async fn main() -> Result<()> {
    set_global_service_name(env!("CARGO_PKG_NAME"));
    crate::logging::init()?;
    let args = Cli::parse();
    match args.command {
        Some(command) => match command {
            cli::CliCommands::Version => println!("{}", version()),
            cli::CliCommands::Cscs {
                command: cscs_command,
            } => match cscs_command {
                cli::CscsCommands::Login => cli_cscs_login().await?,
                cli::CscsCommands::Job { command } => match command {
                    cli::CscsJobCommands::List => cli_cscs_job_list().await?,
                    cli::CscsJobCommands::Get { job_id } => cli_cscs_job_detail(job_id).await?,
                    cli::CscsJobCommands::Submit {
                        script_file,
                        image,
                        command,
                    } => cli_cscs_job_start(script_file, image, command).await?,
                    cli::CscsJobCommands::Cancel { job_id } => cli_cscs_job_cancel(job_id).await?,
                },
                cli::CscsCommands::System { command } => match command {
                    cli::CscsSystemCommands::List => cli_cscs_system_list().await?,
                },
            },
        },
        None => run_tui()?,
    }

    Ok(())
}

fn run_tui() -> Result<()> {
    crate::errors::init()?;
    //we initialize the terminal early so the panic handler that restores the terminal is correctly set up
    let adapter = CrosstermTerminalAdapter::new()?;
    let bridge = TerminalBridge::init(adapter).expect("Cannot initialize terminal");
    let handle = Handle::current();

    let (cscs_device_tx, cscs_device_rx) = mpsc::channel(100);
    let (error_tx, error_rx) = mpsc::channel(100);
    let event_listener = EventListenerCfg::default()
        .with_handle(handle)
        .async_crossterm_input_listener(Duration::default(), 3)
        .add_async_port(
            Box::new(AsyncDeviceFlowPort::new(cscs_device_rx)),
            Duration::from_millis(500),
            1,
        )
        .add_async_port(
            Box::new(AsyncErrorPort::new(error_rx)),
            Duration::default(),
            1,
        )
        .add_async_port(
            Box::new(AsyncFetchWorkloadsPort::new()),
            Duration::from_secs(2),
            1,
        );

    let mut app: Application<Id, Msg, UserEvent> = Application::init(event_listener);

    // subscribe component to clause
    app.mount(Id::Toolbar, Box::new(Toolbar::new()), vec![])?;
    app.mount(Id::WorkloadList, Box::new(WorkloadList::default()), vec![])?;
    app.mount(
        Id::GlobalListener,
        Box::new(GlobalListener::default()),
        vec![
            Sub::new(
                SubEventClause::Keyboard(KeyEvent {
                    code: Key::Char('q'),
                    modifiers: KeyModifiers::NONE,
                }),
                SubClause::Always,
            ),
            Sub::new(
                SubEventClause::Keyboard(KeyEvent {
                    code: Key::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                }),
                SubClause::Always,
            ),
            Sub::new(
                SubEventClause::Discriminant(UserEvent::Info("".to_string())),
                SubClause::Always,
            ),
            Sub::new(
                SubEventClause::Discriminant(UserEvent::Error("".to_string())),
                SubClause::Always,
            ),
            Sub::new(
                SubEventClause::User(UserEvent::Cscs(CscsEvent::LoggedIn)),
                SubClause::Always,
            ),
            Sub::new(
                SubEventClause::Keyboard(KeyEvent {
                    code: Key::Char('x'),
                    modifiers: KeyModifiers::NONE,
                }),
                SubClause::Not(Box::new(SubClause::AndMany(vec![
                    SubClause::IsMounted(Id::Menu),
                    SubClause::IsMounted(Id::ErrorPopup),
                ]))),
            ),
        ],
    )?;

    app.active(&Id::WorkloadList).expect("failed to active");

    let mut model = Model::new(app, bridge, cscs_device_tx, error_tx);
    // Main loop
    // NOTE: loop until quit; quit is set in update if AppClose is received from counter
    while !model.quit {
        // Tick
        match model.app.tick(PollStrategy::Once) {
            Err(err) => {
                panic!("application error {err}");
            }
            Ok(messages) if !messages.is_empty() => {
                // NOTE: redraw if at least one msg has been processed
                model.redraw = true;
                for msg in messages {
                    let mut msg = Some(msg);
                    while msg.is_some() {
                        msg = model.update(msg);
                    }
                }
            }
            _ => {}
        }
        // Redraw
        if model.redraw {
            model.view();
            model.redraw = false;
        }
    }

    model.terminal.restore()?;

    Ok(())
}
