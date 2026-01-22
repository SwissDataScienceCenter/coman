use std::{error::Error, path::PathBuf, str::FromStr, thread};

use clap::{Args, Command, Parser, Subcommand, ValueHint, builder::TypedValueParser};
use clap_complete::{ArgValueCompleter, CompletionCandidate, Generator, Shell, generate};
use color_eyre::{Report, Result};
use itertools::Itertools;
use strum::VariantNames;
use tokio::sync::mpsc;

use crate::{
    config::{ComputePlatform, Config, get_config_dir, get_data_dir, get_project_local_config_file},
    cscs::{
        api_client::{
            client::{EdfSpec as EdfSpecEnum, ScriptSpec as ScriptSpecEnum},
            types::PathType,
        },
        handlers::{cscs_file_list, cscs_job_list, file_system_roots},
    },
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
    Proxy { system: String, job_id: i64 },
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
    #[clap(about = "Get config values")]
    Get {
        #[clap(help = "Config key path, e.g. `cscs.current_system`", value_hint=ValueHint::Other)]
        key_path: String,
    },

    #[clap(about = "Show whole currently active config")]
    Show,
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
    #[clap(
        alias("pf"),
        about = "Forward a local port to a remote port for a job. Note that the port needs to have been exposed with the -P flag on job submission [aliases: pf]"
    )]
    PortForward {
        #[arg(short, long, help = "Local port to forward from")]
        source_port: u16,
        #[arg(short, long, help = "Remote port to forward to")]
        destination_port: u16,
        #[arg(help="id or name of the job (name uses newest job of that name)", add = ArgValueCompleter::new(job_id_or_name_completer))]
        job: JobIdOrName,
    },
}

#[derive(Args, Clone, Debug)]
#[group(multiple = false)]
pub struct ScriptSpec {
    #[arg(
        long,
        help = "generate and upload script file based on template (on by default unless `--local-script` or `--remote-script` are passed)"
    )]
    generate_script: bool,
    #[arg(long, value_name = "PATH", help = "upload local script file", value_hint=ValueHint::FilePath)]
    local_script: Option<PathBuf>,
    #[arg(long, value_name = "PATH", help = "use script file already present on remote", add = ArgValueCompleter::new(remote_path_completer))]
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
        help = "generate and upload edf file based on template (on by default unless `--local-edf` or `--remote-edf` are passed)"
    )]
    generate_edf: bool,
    #[arg(long, value_name = "PATH", help = "upload local edf file", value_hint=ValueHint::FilePath)]
    local_edf: Option<PathBuf>,
    #[arg(long, value_name = "PATH", help = "use edf file already present on remote", add = ArgValueCompleter::new(remote_path_completer))]
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

#[derive(Debug, Clone)]
pub enum JobIdOrName {
    Id(i64),
    Name(String),
}

impl FromStr for JobIdOrName {
    type Err = Report;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(s.parse::<i64>()
            .map(JobIdOrName::Id)
            .unwrap_or_else(|_| JobIdOrName::Name(s.to_string())))
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Subcommand, Debug)]
pub enum CscsJobCommands {
    #[clap(alias("ls"), about = "List all jobs [aliases: ls]")]
    List,
    #[clap(alias("g"), about = "Get metadata for a specific job [aliases: g]")]
    Get {
        #[arg(help="id or name of the job (name uses newest job of that name)", add = ArgValueCompleter::new(job_id_or_name_completer))]
        job: JobIdOrName,
    },
    #[clap(about = "Get the stdout of a job")]
    Log {
        #[clap(short, long, action, help = "whether to get stderr instead of stdout")]
        stderr: bool,
        #[arg(help="id or name of the job (name uses newest job of that name)", add = ArgValueCompleter::new(job_id_or_name_completer))]
        job: JobIdOrName,
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
        #[clap(short='P',
            value_name="TARGET",
            help="Ports to forward from the container",
            value_hint=ValueHint::Other)]
        port_forward: Vec<u16>,
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
        #[clap(help="id or name of the job (name uses newest job of that name)",  add = ArgValueCompleter::new(job_id_or_name_completer))]
        job: JobIdOrName,
    },
}
fn job_id_or_name_completer(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let mut completions = vec![];
    let Some(current) = current.to_str() else {
        return completions;
    };
    let jn = JobIdOrName::from_str(current).unwrap();
    // the tokio shenanigans here are to be able to call async code from this sync method,
    // with an already running async runtime from tokio::main, and getting back the result,
    // all without blocking the async runtime in sync code (hence the extra thread).
    let (send, mut recv) = mpsc::unbounded_channel();
    match jn {
        JobIdOrName::Id(id) => {
            tokio::spawn(async move {
                let jobs = cscs_job_list(None, None).await.unwrap();
                let partial_id = id.to_string();
                let ids: Vec<_> = jobs
                    .iter()
                    .map(|j| (j.id.to_string(), j.name.clone()))
                    .filter(|i| i.0.starts_with(&partial_id))
                    .sorted_by_key(|i| i.0.clone())
                    .collect();
                for (id, name) in ids {
                    send.send(CompletionCandidate::new(id).help(Some(name.into()))).unwrap();
                }
            });
        }
        JobIdOrName::Name(name) => {
            tokio::spawn(async move {
                let jobs = cscs_job_list(None, None).await.unwrap();
                let names: Vec<_> = jobs
                    .into_iter()
                    .map(|j| j.name)
                    .filter(|n| n.starts_with(&name))
                    .sorted()
                    .dedup()
                    .collect();
                for name in names {
                    send.send(CompletionCandidate::new(name)).unwrap();
                }
            });
        }
    }
    let sync_recv = thread::spawn(move || {
        let mut completions = vec![];
        while let Some(candidate) = recv.blocking_recv() {
            completions.push(candidate);
        }
        completions
    });
    let comp = sync_recv.join().unwrap();
    completions.extend(comp);

    completions
}

#[derive(Subcommand, Debug)]
pub enum CscsFileCommands {
    #[clap(alias("ls"), about = "List folders and files in a remote path [aliases: ls]")]
    List {
        #[arg(help ="remote path to list", add = ArgValueCompleter::new(remote_path_completer))]
        path: PathBuf,
    },
    #[clap(alias("rm"), about = "Remove remote files or folders [aliases: rm]")]
    Remove {
        #[arg(help ="remote path to remove", add = ArgValueCompleter::new(remote_path_completer))]
        path: PathBuf,
    },
    #[clap(alias("dl"), about = "Download a remote file [aliases: dl]")]
    Download {
        #[clap(help = "The path in the cluster to download", add = ArgValueCompleter::new(remote_path_completer))]
        remote: PathBuf,
        #[clap(help = "The local path to download the file to", value_hint=ValueHint::AnyPath)]
        local: PathBuf,
    },
    #[clap(alias("ul"), about = "Upload a file to remote storage [aliases: ul]")]
    Upload {
        #[clap(help = "The local path to upload to the cluster", value_hint=ValueHint::AnyPath)]
        local: PathBuf,

        #[clap(help = "the path in the cluster to upload to", add = ArgValueCompleter::new(remote_path_completer))]
        remote: PathBuf,
    },
}

fn remote_path_completer(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let mut completions = vec![];
    let Some(current) = current.to_str() else {
        return completions;
    };

    // the tokio shenanigans here are to be able to call async code from this sync method,
    // with an already running async runtime from tokio::main, and getting back the result,
    // all without blocking the async runtime in sync code (hence the extra thread).
    let (send, mut recv) = mpsc::unbounded_channel();
    if current.is_empty() || current == "/" {
        tokio::spawn(async move {
            let roots = file_system_roots().await;
            if let Ok(roots) = roots {
                for root in roots {
                    send.send(CompletionCandidate::new(root.name.clone())).unwrap();
                }
            }
        });
    } else {
        let current = PathBuf::from(current);
        tokio::spawn(async move {
            let parent = current.parent().unwrap();
            let roots = cscs_file_list(current.clone(), None, None).await;
            if let Ok(roots) = roots {
                for root in roots {
                    if root.path_type == PathType::Directory {
                        // joining with "" ensures trailing slash
                        send.send(CompletionCandidate::new(current.join(root.name.clone()).join("")))
                            .unwrap();
                    } else {
                        send.send(CompletionCandidate::new(current.join(root.name.clone())))
                            .unwrap();
                    }
                }
            } else {
                // file listing only work for full paths, so if we want to complet a partial result, we need
                // to list the parent folder and take it from there
                if let Ok(roots) = cscs_file_list(parent.to_path_buf(), None, None).await {
                    let partial = current.file_name().unwrap().to_string_lossy().into_owned();
                    for root in roots {
                        if root.name.starts_with(&partial) {
                            if root.path_type == PathType::Directory {
                                // joining with "" ensures trailing slash
                                send.send(CompletionCandidate::new(parent.join(root.name.clone()).join("")))
                                    .unwrap();
                            } else {
                                send.send(CompletionCandidate::new(parent.join(root.name))).unwrap();
                            }
                        }
                    }
                }
            }
        });
    }
    let sync_recv = thread::spawn(move || {
        let mut completions = vec![];
        while let Some(candidate) = recv.blocking_recv() {
            completions.push(candidate);
        }
        completions
    });
    let comp = sync_recv.join().unwrap();
    completions.extend(comp);

    completions
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

pub const COMAN_VERSION: &str = env!("CARGO_PKG_VERSION");

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
