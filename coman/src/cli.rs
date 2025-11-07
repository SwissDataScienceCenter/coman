use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::{
    config::{get_config_dir, get_data_dir},
    util::types::DockerImageUrl,
};

#[derive(Subcommand, Debug)]
pub enum CliCommands {
    Version,
    Cscs {
        #[command(subcommand)]
        command: CscsCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum CscsCommands {
    Login,
    #[clap(alias("j"))]
    Job {
        #[command(subcommand)]
        command: CscsJobCommands,
    },
    #[clap(alias("s"))]
    System {
        #[command(subcommand)]
        command: CscsSystemCommands,
    },
}
#[derive(Subcommand, Debug)]
pub enum CscsJobCommands {
    #[clap(alias("ls"))]
    List,
    #[clap(alias("g"))]
    Get { job_id: i64 },
    #[clap(alias("s"))]
    Submit {
        #[clap(short, long)]
        script_file: Option<PathBuf>,
        #[clap(short, long)]
        image: Option<DockerImageUrl>,
        #[clap(short, long, trailing_var_arg = true)]
        command: Option<Vec<String>>,
    },
}
#[derive(Subcommand, Debug)]
pub enum CscsSystemCommands {
    #[clap(alias("ls"))]
    List,
}

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {
    /// Tick rate, i.e. number of ticks per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 4.0)]
    pub tick_rate: f64,

    /// Frame rate, i.e. number of frames per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 60.0)]
    pub frame_rate: f64,

    #[command(subcommand)]
    pub command: Option<CliCommands>,
}

const VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "-",
    env!("VERGEN_GIT_DESCRIBE"),
    " (",
    env!("VERGEN_BUILD_DATE"),
    ")"
);

pub fn version() -> String {
    let author = clap::crate_authors!();

    // let current_exe_path = PathBuf::from(clap::crate_name!()).display().to_string();
    let config_dir_path = get_config_dir().display().to_string();
    let data_dir_path = get_data_dir().display().to_string();

    format!(
        "\
{VERSION_MESSAGE}

Authors: {author}

Config directory: {config_dir_path}
Data directory: {data_dir_path}"
    )
}
