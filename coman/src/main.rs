use std::time::Duration;

use clap::Parser;
use color_eyre::Result;
use keyring::set_global_service_name;
use tokio::{runtime::Handle, sync::mpsc};
use tuirealm::{
    Application, EventListenerCfg, PollStrategy, Sub, SubClause, SubEventClause, Update,
    event::{Key, KeyEvent, KeyModifiers},
    terminal::{CrosstermTerminalAdapter, TerminalBridge},
};

use crate::{
    app::{
        ids::Id,
        messages::{Msg, View},
        model::Model,
        user_events::{CscsEvent, FileEvent, UserEvent},
    },
    cli::{Cli, version},
    components::{file_tree::FileTree, global_listener::GlobalListener, toolbar::Toolbar, workload_list::WorkloadList},
    config::Config,
    cscs::{
        cli::{
            cli_cscs_file_download, cli_cscs_file_list, cli_cscs_file_upload, cli_cscs_job_cancel, cli_cscs_job_detail,
            cli_cscs_job_list, cli_cscs_job_log, cli_cscs_job_start, cli_cscs_login, cli_cscs_set_system,
            cli_cscs_system_list,
        },
        ports::{
            AsyncFetchWorkloadsPort, AsyncFileTreePort, AsyncJobLogPort, AsyncSelectSystemPort, AsyncUserEventPort,
        },
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
                system,
                platform,
            } => match cscs_command {
                cli::CscsCommands::Login => cli_cscs_login().await?,
                cli::CscsCommands::Job { command } => match command {
                    cli::CscsJobCommands::List => cli_cscs_job_list(system, platform).await?,
                    cli::CscsJobCommands::Get { job_id } => cli_cscs_job_detail(job_id, system, platform).await?,
                    cli::CscsJobCommands::Log { job_id } => cli_cscs_job_log(job_id, system, platform).await?,
                    cli::CscsJobCommands::Submit {
                        script_file,
                        image,
                        command,
                        workdir,
                        env,
                    } => cli_cscs_job_start(script_file, image, command, workdir, env, system, platform).await?,
                    cli::CscsJobCommands::Cancel { job_id } => cli_cscs_job_cancel(job_id, system, platform).await?,
                },
                cli::CscsCommands::File { command } => match command {
                    cli::CscsFileCommands::List { path } => cli_cscs_file_list(path, system, platform).await?,
                    cli::CscsFileCommands::Download { remote, local, account } => {
                        cli_cscs_file_download(remote, local, account, system, platform).await?
                    }
                    cli::CscsFileCommands::Upload { local, remote, account } => {
                        cli_cscs_file_upload(local, remote, account, system, platform).await?
                    }
                },
                cli::CscsCommands::System { command } => match command {
                    cli::CscsSystemCommands::List => cli_cscs_system_list(platform).await?,
                    cli::CscsSystemCommands::Set { system_name, global } => {
                        cli_cscs_set_system(system_name, global).await?
                    }
                },
            },
            cli::CliCommands::Init { destination } => Config::create_config(destination)?,
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

    let (select_system_tx, select_system_rx) = mpsc::channel(100);
    let (job_log_tx, job_log_rx) = mpsc::channel(100);
    let (file_tree_tx, file_tree_rx) = mpsc::channel(100);
    let (user_event_tx, user_event_rx) = mpsc::channel(100);
    let (error_tx, error_rx) = mpsc::channel(100);

    // Set up ports that produce events
    // Since the TUI code is synchronous, we set up async ports for async actions that
    // listen on a tokio queue for triggers, do async actions and then produce regular events
    // that the components can handle
    let event_listener = EventListenerCfg::default()
        .with_handle(handle)
        .async_crossterm_input_listener(Duration::default(), 3)
        .add_async_port(Box::new(AsyncErrorPort::new(error_rx)), Duration::default(), 1)
        .add_async_port(Box::new(AsyncFetchWorkloadsPort::new()), Duration::from_secs(2), 1)
        .add_async_port(
            Box::new(AsyncSelectSystemPort::new(select_system_rx)),
            Duration::default(),
            1,
        )
        .add_async_port(Box::new(AsyncJobLogPort::new(job_log_rx)), Duration::from_secs(3), 1)
        .add_async_port(Box::new(AsyncFileTreePort::new(file_tree_rx)), Duration::default(), 1)
        .add_async_port(Box::new(AsyncUserEventPort::new(user_event_rx)), Duration::default(), 1);

    let mut app: Application<Id, Msg, UserEvent> = Application::init(event_listener);

    // Mount components and set up which component get which message
    app.mount(
        Id::Toolbar,
        Box::new(Toolbar::new()),
        vec![Sub::new(
            SubEventClause::Discriminant(UserEvent::SwitchedToView(View::default())),
            SubClause::Always,
        )],
    )?;
    app.mount(
        Id::WorkloadList,
        Box::new(WorkloadList::default()),
        vec![Sub::new(
            SubEventClause::Any,
            SubClause::AndMany(vec![SubClause::IsMounted(Id::WorkloadList), popup_exclusion_clause()]),
        )],
    )?;
    app.mount(
        Id::FileView,
        Box::new(FileTree::new(file_tree_tx.clone())),
        vec![
            Sub::new(
                SubEventClause::Discriminant(UserEvent::File(FileEvent::List("".to_owned(), vec![]))),
                SubClause::Always,
            ),
            Sub::new(
                SubEventClause::Any,
                SubClause::AndMany(vec![SubClause::IsMounted(Id::FileView), popup_exclusion_clause()]),
            ),
        ],
    )?;
    app.mount(
        Id::GlobalListener,
        Box::new(GlobalListener::default()),
        vec![
            Sub::new(
                SubEventClause::Keyboard(KeyEvent {
                    code: Key::Char('q'),
                    modifiers: KeyModifiers::NONE,
                }),
                SubClause::Not(Box::new(SubClause::IsMounted(Id::LoginPopup))),
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
                SubEventClause::Discriminant(UserEvent::File(FileEvent::DownloadSuccessful)),
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
                popup_exclusion_clause(),
            ),
            Sub::new(
                SubEventClause::Keyboard(KeyEvent {
                    code: Key::Char('f'),
                    modifiers: KeyModifiers::NONE,
                }),
                popup_exclusion_clause(),
            ),
            Sub::new(
                SubEventClause::Keyboard(KeyEvent {
                    code: Key::Char('w'),
                    modifiers: KeyModifiers::NONE,
                }),
                popup_exclusion_clause(),
            ),
        ],
    )?;

    app.active(&Id::WorkloadList).expect("failed to active");

    let mut model = Model::new(
        app,
        bridge,
        error_tx,
        select_system_tx,
        job_log_tx,
        user_event_tx,
        file_tree_tx,
    );
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

fn popup_exclusion_clause() -> SubClause<Id> {
    SubClause::Not(Box::new(SubClause::OrMany(vec![
        SubClause::IsMounted(Id::Menu),
        SubClause::IsMounted(Id::ErrorPopup),
        SubClause::IsMounted(Id::LoginPopup),
        SubClause::IsMounted(Id::DownloadPopup),
    ])))
}
