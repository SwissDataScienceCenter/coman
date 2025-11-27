use std::{error::Error, path::PathBuf};

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
    #[clap(about = "Create a new project configuration file")]
    Init {
        #[clap(help = "Destination folder to create config in (default = current directory)")]
        destination: Option<PathBuf>,
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
    Get {
        job_id: i64,
    },
    Log {
        job_id: i64,
    },

    #[clap(alias("s"))]
    Submit {
        #[clap(short, long, help = "the path to the srun script file to use")]
        script_file: Option<PathBuf>,
        #[clap(
            short,
            long,
            help = "the working directory path inside the container (note this is different from the working directory that the srun command is executed from)"
        )]
        workdir: Option<String>,
        #[clap(short='E', value_name="KEY=VALUE", value_parser=parse_key_val::<String,String>, help="Environment variables to set in the container")]
        env: Vec<(String, String)>,
        #[clap(short, long, help = "The docker image to use")]
        image: Option<DockerImageUrl>,
        #[clap(trailing_var_arg = true, help = "The command to run in the container")]
        command: Option<Vec<String>>,
    },
    #[clap(alias("c"))]
    Cancel {
        job_id: i64,
    },
}
#[derive(Subcommand, Debug)]
pub enum CscsSystemCommands {
    #[clap(alias("ls"), about = "List available compute systems")]
    List,
    #[clap(
        alias("s"),
        about = "Set system to use (e.g. `daint`, see `coman cscs ls` for available systems)"
    )]
    Set {
        #[clap(short, long, action, help = "set in global config instead of project-local one")]
        global: bool,
        #[clap(help = "System name to use")]
        system_name: String,
    },
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

fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: Error + Send + Sync + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}
