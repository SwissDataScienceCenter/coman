use std::{error::Error, path::PathBuf};

use clap::{Parser, Subcommand, builder::TypedValueParser};
use strum::VariantNames;

use crate::{
    config::{ComputePlatform, get_config_dir, get_data_dir},
    util::types::DockerImageUrl,
};

#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum CliCommands {
    #[clap(about = "Show version and config file locations")]
    Version,
    #[clap(about = "Subcommands related to CSCS")]
    Cscs {
        #[command(subcommand)]
        command: CscsCommands,
        #[clap(short, long, help = "override compute system (e.g. 'eiger', 'daint')")]
        system: Option<String>,
        #[clap(short, long, ignore_case=true, value_parser=clap::builder::PossibleValuesParser::new(ComputePlatform::VARIANTS).map(|s|s.parse::<ComputePlatform>().unwrap()),help = "override compute platform (one of 'hpc', 'ml' or 'cw')")]
        platform: Option<ComputePlatform>,
        #[clap(short, long, help = "override compute account to use (project or user)")]
        account: Option<String>,
    },
    #[clap(about = "Create a new project configuration file")]
    Init {
        #[clap(help = "Destination folder to create config in (default = current directory)")]
        destination: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
pub enum CscsCommands {
    #[clap(about = "Log in to CSCS")]
    Login,
    #[clap(alias("j"), about = "Job subcommands [aliases: j]")]
    Job {
        #[command(subcommand)]
        command: CscsJobCommands,
    },
    #[clap(alias("f"), about = "File management subcommands [aliases: f]")]
    File {
        #[command(subcommand)]
        command: CscsFileCommands,
    },
    #[clap(
        alias("s"),
        about = "Subcommands for managing for interacting with the compute system config (e.g. 'daint') [aliases: s]"
    )]
    System {
        #[command(subcommand)]
        command: CscsSystemCommands,
    },
}
#[derive(Subcommand, Debug)]
pub enum CscsJobCommands {
    #[clap(alias("ls"), about = "List all jobs [aliases: ls]")]
    List,
    #[clap(alias("g"), about = "Get metadata for a specific job [aliases: g]")]
    Get { job_id: i64 },
    #[clap(about = "Get the stdout of a job")]
    Log { job_id: i64 },

    #[clap(alias("s"), about = "Submit a new compute job [aliases: s]")]
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
    #[clap(
        alias("c"),
        about = "Cancel a running job, fails if the job isn't running [aliases: c]"
    )]
    Cancel { job_id: i64 },
}

#[derive(Subcommand, Debug)]
pub enum CscsFileCommands {
    #[clap(alias("ls"), about = "List folders and files in a remote path [aliases: ls]")]
    List { path: PathBuf },
    #[clap(alias("dl"), about = "Download a remote file [aliases: dl]")]
    Download {
        #[clap(help = "The path in the cluster to download")]
        remote: PathBuf,
        #[clap(help = "The local path to download the file to")]
        local: PathBuf,
    },
    #[clap(alias("ul"), about = "Upload a file to remote storage [aliases: ul]")]
    Upload {
        #[clap(help = "The local path to upload to the cluster")]
        local: PathBuf,

        #[clap(help = "the path in the cluster to upload to")]
        remote: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
pub enum CscsSystemCommands {
    #[clap(alias("ls"), about = "List available compute systems [aliases: ls]")]
    List,
    #[clap(
        alias("s"),
        about = "Set system to use (e.g. `daint`, see `coman cscs ls` for available systems) [aliases: s]"
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
