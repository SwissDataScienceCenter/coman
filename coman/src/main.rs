use std::time::Duration;

use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;
use color_eyre::Result;
use keyring::set_global_service_name;
use self_update::cargo_crate_version;
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
        user_events::{CscsEvent, FileEvent, JobEvent, StatusEvent, UserEvent},
    },
    cli::{
        app::{
            Cli, CliCommands, ConfigCommands, CscsCommands, CscsFileCommands, CscsJobCommands, CscsSystemCommands,
            get_config, print_completions, set_config, version,
        },
        exec::cli_exec_command,
        proxy::cli_proxy_command,
    },
    components::{
        file_tree::FileTree, global_listener::GlobalListener, status_bar::StatusBar, toolbar::Toolbar,
        workload_list::WorkloadList,
    },
    config::Config,
    cscs::{
        api_client::client::JobStartOptions,
        cli::{
            cli_cscs_file_delete, cli_cscs_file_download, cli_cscs_file_list, cli_cscs_file_upload,
            cli_cscs_job_cancel, cli_cscs_job_detail, cli_cscs_job_list, cli_cscs_job_log, cli_cscs_job_resource_usage,
            cli_cscs_job_start, cli_cscs_login, cli_cscs_port_forward, cli_cscs_set_system, cli_cscs_system_list,
        },
        ports::{
            AsyncBackgroundTaskPort, AsyncFetchWorkloadsPort, AsyncJobLogPort, AsyncJobResourceUsagePort,
            AsyncSelectSystemPort, AsyncUserEventPort,
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
    CompleteEnv::with_factory(Cli::command).complete();
    let args = Cli::parse();
    match args.command {
        Some(command) => match command {
            CliCommands::Version => println!("{}", version()),
            CliCommands::Update => {
                tokio::task::spawn_blocking(move || {
                    update().unwrap();
                })
                .await?
            }
            CliCommands::Completions { generator } => {
                let mut cmd = Cli::command();
                print_completions(generator, &mut cmd);
            }
            CliCommands::Config {
                command: config_command,
            } => match config_command {
                ConfigCommands::Set {
                    key_path,
                    value,
                    global,
                } => set_config(key_path, value, global)?,
                ConfigCommands::Get { key_path } => println!("{}", get_config(key_path)?),
                ConfigCommands::Show => {
                    let config = Config::new()?;
                    let content = toml::to_string_pretty(&config.values)?;
                    println!("{}", content)
                }
            },
            CliCommands::Cscs {
                command: cscs_command,
                system,
                platform,
                account,
            } => match cscs_command {
                CscsCommands::Login => cli_cscs_login().await?,
                CscsCommands::Job { command } => match command {
                    CscsJobCommands::List => cli_cscs_job_list(system, platform).await?,
                    CscsJobCommands::Get { job } => cli_cscs_job_detail(job, system, platform).await?,
                    CscsJobCommands::Log { job, stderr } => cli_cscs_job_log(job, stderr, system, platform).await?,
                    CscsJobCommands::Submit {
                        name,
                        image,
                        command,
                        workdir,
                        env,
                        port_forward,
                        mount,
                        stdout,
                        stderr,
                        edf_spec,
                        script_spec,
                        no_ssh,
                        ssh_key,
                        no_coman,
                    } => {
                        cli_cscs_job_start(
                            name,
                            JobStartOptions {
                                image,
                                command,
                                container_workdir: workdir,
                                env,
                                port_forward,
                                mount,
                                stdout,
                                stderr,
                                edf_spec: edf_spec.unwrap_or_default().into(),
                                script_spec: script_spec.unwrap_or_default().into(),
                                no_ssh,
                                ssh_key,
                                no_coman,
                            },
                            system,
                            platform,
                            account,
                        )
                        .await?
                    }
                    CscsJobCommands::Cancel { job } => cli_cscs_job_cancel(job, system, platform).await?,
                    CscsJobCommands::ResourceUsage { job } => {
                        cli_cscs_job_resource_usage(job, system, platform).await?
                    }
                },
                CscsCommands::File { command } => match command {
                    CscsFileCommands::List { path } => cli_cscs_file_list(path, system, platform).await?,
                    CscsFileCommands::Remove { path } => cli_cscs_file_delete(path, system, platform).await?,
                    CscsFileCommands::Download { remote, local } => {
                        cli_cscs_file_download(remote, local, account, system, platform).await?
                    }
                    CscsFileCommands::Upload { local, remote } => {
                        cli_cscs_file_upload(local, remote, account, system, platform).await?
                    }
                },
                CscsCommands::System { command } => match command {
                    CscsSystemCommands::List => cli_cscs_system_list(platform).await?,
                    CscsSystemCommands::Set { system_name, global } => cli_cscs_set_system(system_name, global).await?,
                },
                CscsCommands::PortForward {
                    source_port,
                    destination_port,
                    job,
                } => cli_cscs_port_forward(source_port, destination_port, job, system, platform).await?,
            },
            CliCommands::Init { destination, name } => Config::create_project_config(destination, name)?,
            CliCommands::Exec { command } => cli_exec_command(command).await?,
            CliCommands::Proxy { system, job_id } => cli_proxy_command(system, job_id).await?,
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
    let (job_resource_usage_tx, job_resource_usage_rx) = mpsc::channel(100);
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
            Box::new(AsyncJobResourceUsagePort::new(job_resource_usage_rx)),
            Duration::from_secs(1),
            1,
        )
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
        vec![
            Sub::new(
                SubEventClause::Discriminant(UserEvent::Cscs(CscsEvent::LoggedIn)),
                SubClause::Always,
            ),
            Sub::new(
                SubEventClause::Discriminant(UserEvent::Job(JobEvent::Cancel)),
                SubClause::Always,
            ),
        ],
    )?;
    app.mount(
        Id::FileView,
        Box::new(FileTree::new(background_task_tx.clone())),
        vec![Sub::new(
            SubEventClause::Discriminant(UserEvent::File(FileEvent::DownloadCurrentFile)),
            SubClause::Always,
        )],
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
        job_resource_usage_tx,
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

fn update() -> Result<()> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner("SwissDataScienceCenter")
        .repo_name("coman")
        .bin_name("coman")
        .show_download_progress(true)
        .current_version(cargo_crate_version!())
        .build()?
        .update()?;
    if status.updated() {
        println!("Successfully updated to version: `{}`", status.version());
    } else {
        println!("Already up to date at version: `{}`", status.version());
    }
    Ok(())
}
