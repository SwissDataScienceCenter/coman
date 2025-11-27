use std::{env, io::stdout};

use color_eyre::Result;
use crossterm::event::DisableMouseCapture;
use tokio::sync::mpsc;
use tracing::error;
use tuirealm::{
    Event,
    listener::{ListenerResult, PollAsync},
};

use crate::app::user_events::UserEvent;

pub fn init() -> Result<()> {
    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
        .panic_section(format!(
            "This is a bug. Consider reporting it at {}",
            env!("CARGO_PKG_REPOSITORY")
        ))
        .capture_span_trace_by_default(false)
        .display_location_section(false)
        .display_env_section(false)
        .into_hooks();
    eyre_hook.install()?;
    std::panic::set_hook(Box::new(move |panic_info| {
        crossterm::execute!(stdout(), DisableMouseCapture).unwrap_or_default();
        #[cfg(not(debug_assertions))]
        {
            use human_panic::{handle_dump, metadata, print_msg};
            let metadata = metadata!();
            let file_path = handle_dump(&metadata, panic_info);
            // prints human-panic message
            print_msg(file_path, &metadata).expect("human-panic: printing error message to console failed");
            eprintln!("{}", panic_hook.panic_report(panic_info)); // prints color-eyre stack trace to stderr
        }
        let msg = format!("{}", panic_hook.panic_report(panic_info));
        error!("Error(hook): {}", strip_ansi_escapes::strip_str(msg));

        #[cfg(debug_assertions)]
        {
            // Better Panic stacktrace that is only enabled when debugging.
            better_panic::Settings::auto()
                .most_recent_first(false)
                .lineno_suffix(true)
                .verbosity(better_panic::Verbosity::Full)
                .create_panic_handler()(panic_info);
        }

        std::process::exit(libc::EXIT_FAILURE);
    }));
    Ok(())
}

/// Similar to the `std::dbg!` macro, but generates `tracing` events rather
/// than printing to stdout.
///
/// By default, the verbosity level for the generated events is `DEBUG`, but
/// this can be customized.
#[macro_export]
macro_rules! trace_dbg {
        (target: $target:expr, level: $level:expr, $ex:expr) => {
            {
                match $ex {
                        value => {
                                tracing::event!(target: $target, $level, ?value, stringify!($ex));
                                value
                        }
                }
            }
        };
        (level: $level:expr, $ex:expr) => {
                trace_dbg!(target: module_path!(), level: $level, $ex)
        };
        (target: $target:expr, $ex:expr) => {
                trace_dbg!(target: $target, level: tracing::Level::DEBUG, $ex)
        };
        ($ex:expr) => {
                trace_dbg!(level: tracing::Level::DEBUG, $ex)
        };
}

// Used to allow forwarding errors to the normal event system from async background jobs
pub(crate) struct AsyncErrorPort {
    receiver: mpsc::Receiver<String>,
}

impl AsyncErrorPort {
    pub fn new(receiver: mpsc::Receiver<String>) -> Self {
        Self { receiver }
    }
}

#[tuirealm::async_trait]
impl PollAsync<UserEvent> for AsyncErrorPort {
    async fn poll(&mut self) -> ListenerResult<Option<Event<UserEvent>>> {
        if let Some(error_msg) = self.receiver.recv().await {
            Ok(Some(Event::User(UserEvent::Error(error_msg))))
        } else {
            Ok(None)
        }
    }
}
