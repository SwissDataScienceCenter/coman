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
        user_events::{CscsEvent, FileEvent, StatusEvent, UserEvent},
    },
    cli::{Cli, get_config, set_config, version},
    components::{
        file_tree::FileTree, global_listener::GlobalListener, status_bar::StatusBar, toolbar::Toolbar,
        workload_list::WorkloadList,
    },
    config::Config,
    cscs::{
        api_client::client::JobStartOptions,
        cli::{
            cli_cscs_file_download, cli_cscs_file_list, cli_cscs_file_upload, cli_cscs_job_cancel, cli_cscs_job_detail,
            cli_cscs_job_list, cli_cscs_job_log, cli_cscs_job_start, cli_cscs_login, cli_cscs_set_system,
            cli_cscs_system_list,
        },
        ports::{
            AsyncBackgroundTaskPort, AsyncFetchWorkloadsPort, AsyncJobLogPort, AsyncSelectSystemPort,
            AsyncUserEventPort,
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
            cli::CliCommands::Config {
                command: config_command,
            } => match config_command {
                cli::ConfigCommands::Set {
                    key_path,
                    value,
                    global,
                } => set_config(key_path, value, global)?,
                cli::ConfigCommands::Get { key_path } => println!("{}", get_config(key_path)?),
            },
            cli::CliCommands::Cscs {
                command: cscs_command,
                system,
                platform,
                account,
            } => match cscs_command {
                cli::CscsCommands::Login => cli_cscs_login().await?,
                cli::CscsCommands::Job { command } => match command {
                    cli::CscsJobCommands::List => cli_cscs_job_list(system, platform).await?,
                    cli::CscsJobCommands::Get { job_id } => cli_cscs_job_detail(job_id, system, platform).await?,
                    cli::CscsJobCommands::Log { job_id, stderr } => {
                        cli_cscs_job_log(job_id, stderr, system, platform).await?
                    }
                    cli::CscsJobCommands::Submit {
                        name,
                        image,
                        command,
                        workdir,
                        env,
                        mount,
                        stdout,
                        stderr,
                        edf_spec,
                        script_spec,
                    } => {
                        cli_cscs_job_start(
                            name,
                            JobStartOptions {
                                image,
                                command,
                                container_workdir: workdir,
                                env,
                                mount,
                                stdout,
                                stderr,
                                edf_spec: edf_spec.unwrap_or_default().into(),
                                script_spec: script_spec.unwrap_or_default().into(),
                            },
                            system,
                            platform,
                            account,
                        )
                        .await?
                    }
                    cli::CscsJobCommands::Cancel { job_id } => cli_cscs_job_cancel(job_id, system, platform).await?,
                },
                cli::CscsCommands::File { command } => match command {
                    cli::CscsFileCommands::List { path } => cli_cscs_file_list(path, system, platform).await?,
                    cli::CscsFileCommands::Download { remote, local } => {
                        cli_cscs_file_download(remote, local, account, system, platform).await?
                    }
                    cli::CscsFileCommands::Upload { local, remote } => {
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
            cli::CliCommands::Init { destination, name } => Config::create_project_config(destination, name)?,
        },
        None => run_tui(args.tick_rate)?,
    }

    Ok(())
}

fn run_tui(tick_rate: f64) -> Result<()> {
    crate::errors::init()?;
    //we initialize the terminal early so the panic handler that restores the terminal is correctly set up
    let adapter = CrosstermTerminalAdapter::new()?;
    let bridge = TerminalBridge::init(adapter).expect("Cannot initialize terminal");
    let handle = Handle::current();

    let (select_system_tx, select_system_rx) = mpsc::channel(100);
    let (job_log_tx, job_log_rx) = mpsc::channel(100);
    let (background_task_tx, background_task_rx) = mpsc::channel(100);
    let (user_event_tx, user_event_rx) = mpsc::channel(100);
    let (error_tx, error_rx) = mpsc::channel(100);

    // Set up ports that produce events
    // Since the TUI code is synchronous, we set up async ports for async actions that
    // listen on a tokio queue for triggers, do async actions and then produce regular events
    // that the components can handle
    let event_listener = EventListenerCfg::default()
        .with_handle(handle)
        .tick_interval(Duration::from_millis((1000.0 / tick_rate) as u64))
        .async_crossterm_input_listener(Duration::default(), 3)
        .add_async_port(Box::new(AsyncErrorPort::new(error_rx)), Duration::default(), 1)
        .add_async_port(Box::new(AsyncFetchWorkloadsPort::new()), Duration::from_secs(2), 1)
        .add_async_port(
            Box::new(AsyncSelectSystemPort::new(select_system_rx)),
            Duration::default(),
            1,
        )
        .add_async_port(Box::new(AsyncJobLogPort::new(job_log_rx)), Duration::from_secs(3), 1)
        .add_async_port(
            Box::new(AsyncBackgroundTaskPort::new(background_task_rx, user_event_tx.clone())),
            Duration::default(),
            1,
        )
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
        Id::StatusBar,
        Box::new(StatusBar::new()),
        vec![
            Sub::new(
                SubEventClause::Discriminant(UserEvent::Status(StatusEvent::Info("".to_owned()))),
                SubClause::Always,
            ),
            Sub::new(SubEventClause::Tick, SubClause::Always),
            Sub::new(
                SubEventClause::Discriminant(UserEvent::Cscs(CscsEvent::SystemSelected("".to_owned()))),
                SubClause::Always,
            ),
        ],
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
        Box::new(FileTree::new(background_task_tx.clone())),
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
        background_task_tx,
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
        SubClause::IsMounted(Id::SystemSelectPopup),
    ])))
}
