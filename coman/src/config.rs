#![allow(dead_code)] // Remove this once you start using the code

use std::{env, path::PathBuf};

use color_eyre::Result;
use directories::ProjectDirs;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

const DEFAULT_CONFIG_TOML: &str = include_str!("../.config/config.toml");

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub data_dir: PathBuf,
    #[serde(default)]
    pub config_dir: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CscsConfig {
    #[serde(default)]
    pub system: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub sbatch_script_template: String,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub edf_file_template: String,
    #[serde(default)]
    pub command: Vec<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default, flatten)]
    pub config: AppConfig,
    #[serde(default)]
    pub cscs: CscsConfig,
}

lazy_static! {
    pub static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_uppercase().to_string();
    pub static ref DATA_FOLDER: Option<PathBuf> =
        env::var(format!("{}_DATA", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
    pub static ref CONFIG_FOLDER: Option<PathBuf> =
        env::var(format!("{}_CONFIG", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
}

impl Config {
    pub fn new() -> Result<Self, config::ConfigError> {
        let data_dir = get_data_dir();
        let config_dir = get_config_dir();
        let mut builder = config::Config::builder()
            .add_source(config::File::from_str(
                DEFAULT_CONFIG_TOML,
                config::FileFormat::Toml,
            ))
            .set_default("data_dir", data_dir.to_str().unwrap())?
            .set_default("config_dir", config_dir.to_str().unwrap())?;

        let config_files = [
            (
                format!("{}.toml", PROJECT_NAME.to_lowercase()),
                config::FileFormat::Toml,
            ),
            (
                format!("{}.json5", PROJECT_NAME.to_lowercase()),
                config::FileFormat::Json5,
            ),
            (
                format!("{}.json", PROJECT_NAME.to_lowercase()),
                config::FileFormat::Json,
            ),
            (
                format!("{}.yaml", PROJECT_NAME.to_lowercase()),
                config::FileFormat::Yaml,
            ),
            (
                format!("{}.ini", PROJECT_NAME.to_lowercase()),
                config::FileFormat::Ini,
            ),
        ];
        for (file, format) in &config_files {
            let source = config::File::from(config_dir.join(file))
                .format(*format)
                .required(false);
            builder = builder.add_source(source);
        }

        // find config override in current directory
        let mut search_path = std::env::current_dir().expect("current directory does not exist");
        loop {
            for (file, format) in &config_files {
                if search_path.join(file).exists() {
                    let source = config::File::from(search_path.join(file))
                        .format(*format)
                        .required(false);
                    builder = builder.add_source(source);
                    break;
                }
            }
            if let Some(p) = search_path.parent() {
                search_path = p.to_path_buf();
            } else {
                break;
            }
        }

        let cfg: Self = builder.build()?.try_deserialize()?;
        Ok(cfg)
    }
}

pub fn get_data_dir() -> PathBuf {
    if let Some(s) = DATA_FOLDER.clone() {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".data")
    }
}

pub fn get_config_dir() -> PathBuf {
    if let Some(s) = CONFIG_FOLDER.clone() {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.config_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".config")
    }
}

fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("ch", "sdsc", env!("CARGO_PKG_NAME"))
}
