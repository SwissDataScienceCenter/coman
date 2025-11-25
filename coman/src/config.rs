#![allow(dead_code)] // Remove this once you start using the code

use std::{collections::HashMap, env, path::PathBuf};

use color_eyre::Result;
use directories::ProjectDirs;
use eyre::eyre;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::trace_dbg;

const DEFAULT_CONFIG_TOML: &str = include_str!("../.config/config.toml");

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SystemDescription {
    pub architecture: Vec<String>,
}

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
    pub current_system: String,
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub sbatch_script_template: String,
    #[serde(default)]
    pub workdir: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub edf_file_template: String,
    #[serde(default)]
    pub command: Vec<String>,

    #[serde(default)]
    pub systems: HashMap<String, SystemDescription>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, flatten)]
    pub config: AppConfig,
    #[serde(default)]
    pub cscs: CscsConfig,
}

lazy_static! {
    pub static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_uppercase().to_string();
    pub static ref DATA_FOLDER: Option<PathBuf> = env::var(format!("{}_DATA", PROJECT_NAME.clone()))
        .ok()
        .map(PathBuf::from);
    pub static ref CONFIG_FOLDER: Option<PathBuf> = env::var(format!("{}_CONFIG", PROJECT_NAME.clone()))
        .ok()
        .map(PathBuf::from);
    pub static ref CONFIG_FILE_NAME: String = format!("{}.toml", PROJECT_NAME.to_lowercase().clone());
    pub static ref CONFIG_FORMAT: config::FileFormat = config::FileFormat::Toml;
}

impl Config {
    pub fn new() -> Result<Self> {
        let builder = default_config_builder()?;
        let builder = global_config_builder(builder)?;
        let builder = project_local_config_builder(builder)?;

        let cfg: Self = builder.build()?.try_deserialize()?;
        Ok(cfg)
    }
    pub fn new_global() -> Result<Self> {
        let builder = default_config_builder()?;
        let builder = global_config_builder(builder)?;

        let cfg: Self = builder.build()?.try_deserialize()?;
        Ok(cfg)
    }

    pub fn write_local(&self) -> Result<()> {
        match get_project_local_config_file() {
            Some(path) => {
                let content = toml::to_string_pretty(self)?;
                std::fs::write(path, content)?;
                Ok(())
            }
            None => Err(eyre!(
                "No config file exists in current project. Consider creating one using '{} init",
                PROJECT_NAME.to_lowercase().clone()
            )),
        }
    }

    pub fn write_global(&self) -> Result<()> {
        let config_dir = get_config_dir();
        let path = config_dir.join(CONFIG_FILE_NAME.clone());
        let content = toml::to_string_pretty(self)?;
        let parent = path.parent().unwrap();
        std::fs::create_dir_all(parent)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn create_config(destination: Option<PathBuf>) -> Result<()> {
        let mut config = Config::new()?;
        let project_dir = destination
            .unwrap_or(std::env::current_dir().expect("current directory does not exist"))
            .canonicalize()?;
        if !project_dir.exists() || !project_dir.is_dir() {
            return Err(eyre!(
                "destination must exist and be a directory, got {}",
                project_dir.to_string_lossy()
            ));
        }

        let name = project_dir
            .file_name()
            .expect("could not get base name from destination");
        config.name = Some(name.to_string_lossy().to_string());

        let config_path = project_dir.join(CONFIG_FILE_NAME.clone());
        std::fs::write(config_path.clone(), "")?;
        let content = toml::to_string_pretty(&config)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }
}

pub fn default_config_builder() -> Result<config::ConfigBuilder<config::builder::DefaultState>> {
    let data_dir = get_data_dir();
    let config_dir = get_config_dir();
    let builder = config::Config::builder()
        .add_source(config::File::from_str(DEFAULT_CONFIG_TOML, config::FileFormat::Toml))
        .set_default("data_dir", data_dir.to_str().unwrap())?
        .set_default("config_dir", config_dir.to_str().unwrap())?;
    Ok(builder)
}

pub fn global_config_builder(
    builder: config::ConfigBuilder<config::builder::DefaultState>,
) -> Result<config::ConfigBuilder<config::builder::DefaultState>> {
    let config_dir = get_config_dir();
    let source = config::File::from(config_dir.join(CONFIG_FILE_NAME.clone()))
        .format(*CONFIG_FORMAT)
        .required(false);
    let builder = builder.add_source(source);
    Ok(builder)
}

pub fn project_local_config_builder(
    builder: config::ConfigBuilder<config::builder::DefaultState>,
) -> Result<config::ConfigBuilder<config::builder::DefaultState>> {
    if let Some(config_path) = get_project_local_config_file() {
        let source = config::File::from(config_path).format(*CONFIG_FORMAT).required(false);
        let builder = builder.add_source(source);
        return Ok(builder);
    }
    Ok(builder)
}

pub fn get_project_local_config_file() -> Option<PathBuf> {
    let mut search_path = std::env::current_dir().expect("current directory does not exist");
    loop {
        if search_path.join(CONFIG_FILE_NAME.clone()).exists() {
            return Some(search_path.join(CONFIG_FILE_NAME.clone()));
        }
        if let Some(p) = search_path.parent() {
            search_path = p.to_path_buf();
        } else {
            break;
        }
    }
    None
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
