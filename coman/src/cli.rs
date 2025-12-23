use std::{error::Error, path::PathBuf, thread, time::Duration};

use base64::prelude::*;
use clap::{Args, Command, Parser, Subcommand, ValueHint, builder::TypedValueParser};
use clap_complete::{Generator, Shell, generate};
use color_eyre::{Result, eyre::eyre};
use iroh_ssh::IrohSsh;
use pid1::Pid1Settings;
use rust_supervisor::{ChildType, Supervisor, SupervisorConfig};
use strum::VariantNames;

use crate::{
    config::{ComputePlatform, Config, get_config_dir, get_data_dir, get_project_local_config_file},
    cscs::api_client::client::{EdfSpec as EdfSpecEnum, ScriptSpec as ScriptSpecEnum},
    util::types::DockerImageUrl,
};

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {
    /// Tick rate, i.e. number of ticks per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 4.0)]
    pub tick_rate: f64,

    #[command(subcommand)]
    pub command: Option<CliCommands>,
}

#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum CliCommands {
    #[clap(about = "Show version and config file locations")]
    Version,
    #[clap(about = "Subcommands related to CSCS")]
    Cscs {
        #[command(subcommand)]
        command: CscsCommands,
        #[clap(short, long, help = "override compute system (e.g. 'eiger', 'daint')", value_hint=ValueHint::Other)]
        system: Option<String>,
        #[clap(
            short,
            long,
            ignore_case=true,
            value_parser=clap::builder::PossibleValuesParser::new(ComputePlatform::VARIANTS).map(
                |s|s.parse::<ComputePlatform>().unwrap()),
            help = "override compute platform (one of 'hpc', 'ml' or 'cw')",
            value_hint=ValueHint::Other)]
        platform: Option<ComputePlatform>,
        #[clap(short, long, help = "override compute account to use (project or user)",value_hint=ValueHint::Other)]
        account: Option<String>,
    },
    #[clap(about = "Create a new project configuration file")]
    Init {
        #[clap(help = "destination folder to create config in (default = current directory)",value_hint=ValueHint::DirPath)]
        destination: Option<PathBuf>,
        #[clap(help = "project name to use", value_hint=ValueHint::Other)]
        name: Option<String>,
    },
    #[clap(about = "Manage configuration")]
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    #[clap(about = "Generate shell completions")]
    Completions {
        /// generate shell completions
        #[clap(value_enum)]
        generator: Shell,
    },
    #[clap(about = "Execute a process/command through coman, with additional monitoring and side processes")]
    Exec {
        #[clap(trailing_var_arg = true, help = "The command to run", value_hint=ValueHint::Other)]
        command: Vec<String>,
    },
    #[clap(hide = true)]
    Proxy { job_id: i64 },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    #[clap(about = "Set config values")]
    Set {
        #[clap(
            short,
            long,
            action,
            help = "whether to change the global config or the project local one"
        )]
        global: bool,
        #[clap(help = "Config key path, e.g. `cscs.current_system`", value_hint=ValueHint::Other)]
        key_path: String,
        #[clap(help = "Value to set", value_parser = parse_toml_value, value_hint=ValueHint::Other)]
        value: toml_edit::Value,
    },
    Get {
        #[clap(help = "Config key path, e.g. `cscs.current_system`", value_hint=ValueHint::Other)]
        key_path: String,
    },
}

#[allow(clippy::large_enum_variant)]
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

#[derive(Args, Clone, Debug)]
#[group(multiple = false)]
pub struct ScriptSpec {
    #[arg(
        long,
        help = "generate and upload script file based on template (on by default unless `--local` or `--remote` are passed)"
    )]
    generate_script: bool,
    #[arg(long, value_name = "PATH", help = "upload local script file", value_hint=ValueHint::FilePath)]
    local_script: Option<PathBuf>,
    #[arg(long, value_name = "PATH", help = "use script file already present on remote", value_hint=ValueHint::Other)]
    remote_script: Option<PathBuf>,
}
impl Default for ScriptSpec {
    fn default() -> Self {
        Self {
            generate_script: true,
            local_script: Default::default(),
            remote_script: Default::default(),
        }
    }
}

impl From<ScriptSpec> for ScriptSpecEnum {
    fn from(val: ScriptSpec) -> Self {
        if let Some(local_script) = val.local_script {
            ScriptSpecEnum::Local(local_script)
        } else if let Some(remote_script) = val.remote_script {
            ScriptSpecEnum::Remote(remote_script)
        } else {
            ScriptSpecEnum::Generate
        }
    }
}

#[derive(Args, Clone, Debug)]
#[group(multiple = false)]
pub struct EdfSpec {
    #[arg(
        long,
        help = "generate and upload edf file based on template (on by default unless `--local` or `--remote` are passed)"
    )]
    generate_edf: bool,
    #[arg(long, value_name = "PATH", help = "upload local edf file", value_hint=ValueHint::FilePath)]
    local_edf: Option<PathBuf>,
    #[arg(long, value_name = "PATH", help = "use edf file already present on remote", value_hint=ValueHint::Other)]
    remote_edf: Option<PathBuf>,
}

impl Default for EdfSpec {
    fn default() -> Self {
        Self {
            generate_edf: true,
            local_edf: Default::default(),
            remote_edf: Default::default(),
        }
    }
}

impl From<EdfSpec> for EdfSpecEnum {
    fn from(val: EdfSpec) -> Self {
        if let Some(local_edf) = val.local_edf {
            EdfSpecEnum::Local(local_edf)
        } else if let Some(remote_edf) = val.remote_edf {
            EdfSpecEnum::Remote(remote_edf)
        } else {
            EdfSpecEnum::Generate
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Subcommand, Debug)]
pub enum CscsJobCommands {
    #[clap(alias("ls"), about = "List all jobs [aliases: ls]")]
    List,
    #[clap(alias("g"), about = "Get metadata for a specific job [aliases: g]")]
    Get {
        #[arg(help="id of the job", value_hint=ValueHint::Other)]
        job_id: i64,
    },
    #[clap(about = "Get the stdout of a job")]
    Log {
        #[clap(short, long, action, help = "whether to get stderr instead of stdout")]
        stderr: bool,
        #[arg(help="id of the job", value_hint=ValueHint::Other)]
        job_id: i64,
    },

    #[clap(alias("s"), about = "Submit a new compute job [aliases: s]")]
    Submit {
        #[clap(short, long, help = "name of the job", value_hint=ValueHint::Other)]
        name: Option<String>,
        #[clap(
            short,
            long,
            help = "the working directory path inside the container (note this is different from the working directory that the srun command is executed from)",
            value_hint=ValueHint::Other
        )]
        workdir: Option<String>,
        #[clap(short='E',
            value_name="KEY=VALUE",
            value_parser=parse_key_val::<String, String>,
            help="Environment variables to set in the container",
            value_hint=ValueHint::Other)]
        env: Vec<(String, String)>,
        #[clap(short='M',
            value_name="PATH:CONTAINER_PATH",
            value_parser=parse_key_val_colon::<String,String>,
            help="Paths to mount inside container",
            value_hint=ValueHint::Other)]
        mount: Vec<(String, String)>,
        #[clap(short, long, help = "The docker image to use", value_hint=ValueHint::Other)]
        image: Option<DockerImageUrl>,
        #[clap(long, help = "Path where stdout of the job gets written to", value_hint=ValueHint::Other)]
        stdout: Option<PathBuf>,
        #[clap(long, help = "Path where stderr of the job gets written to", value_hint=ValueHint::Other)]
        stderr: Option<PathBuf>,
        #[command(flatten)]
        edf_spec: Option<EdfSpec>,
        #[command(flatten)]
        script_spec: Option<ScriptSpec>,
        #[clap(long, action, help = "don't set up ssh integration")]
        no_ssh: bool,
        #[clap(short, long, help="ssh public key to use", value_hint=ValueHint::FilePath)]
        ssh_key: Option<PathBuf>,
        #[clap(long, action, help = "don't upload and inject coman into the container")]
        no_coman: bool,
        #[clap(trailing_var_arg = true, help = "The command to run in the container", value_hint=ValueHint::Other)]
        command: Option<Vec<String>>,
    },
    #[clap(
        alias("c"),
        about = "Cancel a running job, fails if the job isn't running [aliases: c]"
    )]
    Cancel {
        #[clap(help="id of the job", value_hint=ValueHint::Other)]
        job_id: i64,
    },
}

#[derive(Subcommand, Debug)]
pub enum CscsFileCommands {
    #[clap(alias("ls"), about = "List folders and files in a remote path [aliases: ls]")]
    List {
        #[arg(help ="remote path to list", value_hint=ValueHint::Other)]
        path: PathBuf,
    },
    #[clap(alias("dl"), about = "Download a remote file [aliases: dl]")]
    Download {
        #[clap(help = "The path in the cluster to download", value_hint=ValueHint::Other)]
        remote: PathBuf,
        #[clap(help = "The local path to download the file to", value_hint=ValueHint::AnyPath)]
        local: PathBuf,
    },
    #[clap(alias("ul"), about = "Upload a file to remote storage [aliases: ul]")]
    Upload {
        #[clap(help = "The local path to upload to the cluster", value_hint=ValueHint::AnyPath)]
        local: PathBuf,

        #[clap(help = "the path in the cluster to upload to", value_hint=ValueHint::Other)]
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
        #[clap(help = "System name to use", value_hint=ValueHint::Other)]
        system_name: String,
    },
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
    // let current_exe_path = PathBuf::from(clap::crate_name!()).display().to_string();
    let config_dir_path = get_config_dir().display().to_string();
    let data_dir_path = get_data_dir().display().to_string();
    let project_config_dir = get_project_local_config_file()
        .map(|p| p.display().to_string())
        .unwrap_or("".to_owned());

    format!(
        "\
{VERSION_MESSAGE}

Project config directory: {project_config_dir}
Config directory: {config_dir_path}
Data directory: {data_dir_path}"
    )
}

pub fn set_config<V: Into<toml_edit::Value>>(key_path: String, value: V, global: bool) -> Result<()> {
    let mut config = Config::new()?;
    config.set(&key_path, value, global)?;
    Ok(())
}

pub fn get_config(key_path: String) -> Result<String> {
    let config = Config::new()?;
    config.get(&key_path)
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
fn parse_key_val_colon<T, U>(s: &str) -> Result<(T, U), Box<dyn Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: Error + Send + Sync + 'static,
{
    let pos = s
        .find(':')
        .ok_or_else(|| format!("invalid KEY:value: no `:` found in `{s}`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}

pub fn parse_toml_value(value_str: &str) -> Result<toml_edit::Value, toml_edit::TomlError> {
    match value_str.parse() {
        Ok(value) => Ok(value),
        Err(_) if is_bare_string(value_str) => Ok(value_str.into()),
        Err(err) => Err(err),
    }
}
fn is_bare_string(value_str: &str) -> bool {
    // leading whitespace isn't ignored when parsing TOML value expression, but
    // "\n[]" doesn't look like a bare string.
    let trimmed = value_str.trim_ascii().as_bytes();
    if let (Some(&first), Some(&last)) = (trimmed.first(), trimmed.last()) {
        // string, array, or table constructs?
        !matches!(first, b'"' | b'\'' | b'[' | b'{') && !matches!(last, b'"' | b'\'' | b']' | b'}')
    } else {
        true // empty or whitespace only
    }
}

pub fn print_completions<G: Generator>(generator: G, cmd: &mut Command) {
    generate(generator, cmd, cmd.get_name().to_string(), &mut std::io::stdout());
}

/// Runs a wrapped command in a container-safe way and potentially runs background processes like iroh-ssh
pub(crate) async fn cli_exec_command(command: Vec<String>) -> Result<()> {
    // Pid1 takes care of proper terminating of processes and signal handling when running in a container
    Pid1Settings::new()
        .enable_log(true)
        .timeout(Duration::from_secs(2))
        .launch()
        .expect("Launch failed");

    let mut supervisor = Supervisor::new(SupervisorConfig::default());
    supervisor.add_process("iroh-ssh", ChildType::Permanent, || {
        thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("couldn't start tokio");

            // Call the asynchronous connect method using the runtime.
            rt.block_on(async move {
                let mut builder = IrohSsh::builder().accept_incoming(true).accept_port(15263);
                if let Ok(secret) = std::env::var("COMAN_IROH_SECRET") {
                    let secret_key = BASE64_STANDARD.decode(secret).unwrap();
                    let secret_key: &[u8; 32] = secret_key[0..32].try_into().unwrap();
                    builder = builder.secret_key(secret_key);
                }

                let server = builder.build().await.expect("couldn't create iroh server");
                println!("{}@{}", whoami::username(), server.node_id());
                loop {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
            });
        })
    });
    supervisor.add_process("main-process", ChildType::Temporary, move || {
        let command = command.clone();
        thread::spawn(move || {
            let mut child = std::process::Command::new(command[0].clone())
                .args(&command[1..])
                .spawn()
                .expect("Failed to start compute job");
            child.wait().expect("Failed to wait on compute job");
        })
    });

    let supervisor = supervisor.start_monitoring();
    loop {
        thread::sleep(Duration::from_secs(1));

        if let Some(rust_supervisor::ProcessState::Failed | rust_supervisor::ProcessState::Stopped) =
            supervisor.get_process_state("main-process")
        {
            break;
        }
    }
    Ok(())
}

/// Thin wrapper around iroh proxy
pub(crate) async fn cli_proxy_command(job_id: i64) -> Result<()> {
    let data_dir = get_data_dir();
    let endpoint_id = std::fs::read_to_string(data_dir.join(format!("{}.endpoint", job_id)))?;
    println!("{}", endpoint_id);
    iroh_ssh::api::proxy_mode(iroh_ssh::ProxyArgs { node_id: endpoint_id })
        .await
        .map_err(|e| eyre!("couldn't proxy ssh connection: {:?}", e))
}
