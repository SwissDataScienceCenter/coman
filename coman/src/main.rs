use clap::Parser;
use cli::Cli;
use color_eyre::Result;

use crate::app::App;

mod action;
mod api_client;
mod app;
mod cli;
mod components;
mod config;
mod errors;
mod focus_manager;
mod logging;
mod tui;
mod util;

#[tokio::main]
async fn main() -> Result<()> {
    crate::errors::init()?;
    crate::logging::init()?;

    let args = Cli::parse();
    let mut app = App::new(args.tick_rate, args.frame_rate)?;
    app.run().await?;
    Ok(())
}
