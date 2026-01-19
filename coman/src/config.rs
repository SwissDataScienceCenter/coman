#![allow(dead_code)] // Remove this once you start using the code

use std::{collections::HashMap, env, path::PathBuf};

use color_eyre::{
    Result,
    eyre::{Context, ContextCompat, eyre},
};
use directories::ProjectDirs;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use strum_macros::{EnumString, VariantNames};
use toml_edit::DocumentMut;

const DEFAULT_CONFIG_TOML: &str = include_str!("../.config/config.toml");

const DEFAULT_KEYS: &[&str] = &["name", "cscs.account"];

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

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SystemDescription {
    pub architecture: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, strum::Display, EnumString, VariantNames)]
#[strum(serialize_all = "lowercase")]
#[allow(clippy::upper_case_acronyms)]
pub enum ComputePlatform {
    #[default]
    HPC,
    ML,
    CW,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CscsConfig {
    #[serde(default)]
    pub current_system: String,
    #[serde(default)]
    pub current_platform: ComputePlatform,
    #[serde(default)]
    pub account: Option<String>,
    #[serde(default)]
    pub sbatch_script_template: String,
    #[serde(default)]
    pub workdir: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub port_forward: Vec<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub edf_file_template: String,
    #[serde(default)]
    pub ssh_key: Option<PathBuf>,
    #[serde(default)]
    pub command: Vec<String>,

    #[serde(default)]
    pub systems: HashMap<String, SystemDescription>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ComanConfig {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub coman_squash_path: Option<PathBuf>,
    #[serde(default)]
    pub cscs: CscsConfig,
}

#[derive(Clone, Debug)]
pub struct Layer {
    source: PathBuf,
    data: DocumentMut,
}

impl Layer {
    pub fn from_path(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self {
                source: path,
                data: DocumentMut::new(),
            });
        }
        if !path.is_file() {
            return Err(eyre!("Config path {} is not a file", path.display()));
        }
        let content =
            std::fs::read_to_string(path.clone()).wrap_err(format!("couldn't read config {}", path.display()))?;
        let doc = content
            .parse::<DocumentMut>()
            .wrap_err(format!("couldn't parse toml file {}", path.display()))?;
        Ok(Self {
            source: path,
            data: doc,
        })
    }

    pub fn get(&self, key_path: &str) -> Result<Option<String>> {
        let key_path_parsed = toml_edit::Key::parse(key_path)?;
        let root = self.data.as_item();
        let item = lookup_entry(key_path_parsed, root)?;
        let item = item
            .map(|i| i.clone().into_value().map_err(|e| eyre!("{:?}", e)))
            .transpose()
            .wrap_err("couldn't convert config item to value")?;

        Ok(item.map(|val| match val {
            toml_edit::Value::String(v) => v.into_value(),
            toml_edit::Value::Integer(_)
            | toml_edit::Value::Float(_)
            | toml_edit::Value::Boolean(_)
            | toml_edit::Value::Datetime(_)
            | toml_edit::Value::Array(_)
            | toml_edit::Value::InlineTable(_) => val.decorated("", "").to_string(),
        }))
    }

    pub fn set<V: Into<toml_edit::Value>>(&mut self, key_path: &str, value: V) -> Result<()> {
        let key_path_parsed = toml_edit::Key::parse(key_path)?;
        let (leaf, keys) = key_path_parsed.split_last().wrap_err("couldn't parse key path")?;
        let root_table: &mut dyn toml_edit::TableLike = self.data.as_table_mut();
        let table = keys
            .iter()
            .enumerate()
            .try_fold(root_table, |table: &mut dyn toml_edit::TableLike, (i, key)| {
                let sub_item = table.entry_format(key).or_insert_with(implicit_table);
                sub_item.as_table_like_mut().ok_or(&keys[..=i])
            })
            .map_err(|e| eyre!("{:?}", e))
            .wrap_err("couldn't get config item path")?;

        match table.entry_format(leaf) {
            toml_edit::Entry::Occupied(mut occupied_entry) => {
                if !occupied_entry.get().is_value() {
                    return Err(eyre!("would overwrite entry {}", key_path));
                }
                occupied_entry.insert(toml_edit::value(value));
            }
            toml_edit::Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(toml_edit::value(value));
            }
        }

        Ok(())
    }

    pub fn write(&self) -> Result<()> {
        let contents = self.data.to_string();
        std::fs::create_dir_all(self.source.parent().unwrap())?;
        std::fs::write(&self.source, contents).wrap_err(format!("couldn't write config {}", self.source.display()))
    }
}

fn lookup_entry(key_path_parsed: Vec<toml_edit::Key>, root: &toml_edit::Item) -> Result<Option<&toml_edit::Item>> {
    let mut cur_item = root;
    for key in key_path_parsed {
        let Some(table) = cur_item.as_table_like() else {
            return Err(eyre!("couldn't get subentry for {}", cur_item));
        };
        cur_item = match table.get(key.get()) {
            Some(item) => item,
            None => return Ok(None),
        };
    }
    Ok(Some(cur_item))
}

fn implicit_table() -> toml_edit::Item {
    let mut table = toml_edit::Table::new();
    table.set_implicit(true);
    toml_edit::Item::Table(table)
}

#[derive(Clone, Debug)]
pub struct Config {
    pub values: ComanConfig,
    default_layer: toml_edit::DocumentMut,
    global_layer: Layer,
    project_layer: Option<Layer>,
}

impl Config {
    pub fn new() -> Result<Self> {
        let default_layer: DocumentMut = DEFAULT_CONFIG_TOML.parse()?;
        let global_layer = global_config_layer()?;
        let project_layer = project_local_config_layer()?;
        let mut builder =
            config::Config::builder().add_source(config::File::from_str(DEFAULT_CONFIG_TOML, config::FileFormat::Toml));
        builder = builder.add_source(config::File::from_str(
            &global_layer.data.to_string(),
            config::FileFormat::Toml,
        ));
        if let Some(project_layer) = project_layer.clone() {
            builder = builder.add_source(config::File::from_str(
                &project_layer.data.to_string(),
                config::FileFormat::Toml,
            ));
        }

        let cfg: ComanConfig = builder.build()?.try_deserialize()?;

        Ok(Self {
            values: cfg,
            default_layer,
            global_layer,
            project_layer,
        })
    }
    pub fn create_project_config(destination: Option<PathBuf>, name: Option<String>) -> Result<()> {
        let project_dir = destination
            .unwrap_or(std::env::current_dir().expect("current directory does not exist"))
            .canonicalize()?;
        if !project_dir.exists() || !project_dir.is_dir() {
            return Err(eyre!(
                "destination must exist and be a directory, got {}",
                project_dir.to_string_lossy()
            ));
        }

        let name = name.unwrap_or(
            project_dir
                .file_name()
                .expect("could not get base name from destination")
                .to_os_string()
                .into_string()
                .map_err(|e| eyre!("couldn't parse path: {:?}", e))?,
        );

        let config_path = project_dir.join(CONFIG_FILE_NAME.clone());
        let mut project_layer = Layer::from_path(config_path)?;
        project_layer.set("name", name)?;
        project_layer.write()
    }

    pub fn set<V: Into<toml_edit::Value>>(&mut self, key_path: &str, value: V, global: bool) -> Result<()> {
        match global {
            true => {
                self.global_layer.set(key_path, value)?;
                self.validate()?;
                self.global_layer.write()?;
            }
            false => match self.project_layer {
                Some(ref mut layer) => {
                    layer.set(key_path, value)?;
                }
                None => return Err(eyre!("No project config found, please create one with `coman init`")),
            },
        };
        self.validate()?;
        match global {
            true => {
                self.global_layer.write()?;
            }
            false => {
                self.project_layer.as_ref().unwrap().write()?;
            }
        }

        // reload config
        let mut builder =
            config::Config::builder().add_source(config::File::from_str(DEFAULT_CONFIG_TOML, config::FileFormat::Toml));
        builder = builder.add_source(config::File::from_str(
            &self.global_layer.data.to_string(),
            config::FileFormat::Toml,
        ));
        if let Some(project_layer) = self.project_layer.clone() {
            builder = builder.add_source(config::File::from_str(
                &project_layer.data.to_string(),
                config::FileFormat::Toml,
            ));
        }
        self.values = builder.build()?.try_deserialize()?;
        Ok(())
    }

    pub fn get(&self, key_path: &str) -> Result<String> {
        if let Some(ref layer) = self.project_layer {
            match layer.get(key_path) {
                Ok(Some(val)) => return Ok(val),
                Ok(None) => {}
                Err(e) => return Err(e),
            }
        }

        match self.global_layer.get(key_path) {
            Ok(Some(val)) => return Ok(val),
            Ok(None) => {}
            Err(e) => return Err(e),
        };

        let key_path_parsed = toml_edit::Key::parse(key_path)?;
        let item = lookup_entry(key_path_parsed, self.default_layer.as_item())?;
        Ok(item.map(|i| i.to_string()).unwrap_or("".to_owned()))
    }

    pub fn validate(&mut self) -> Result<()> {
        let mut builder =
            config::Config::builder().add_source(config::File::from_str(DEFAULT_CONFIG_TOML, config::FileFormat::Toml));
        builder = builder.add_source(config::File::from_str(
            &self.global_layer.data.to_string(),
            config::FileFormat::Toml,
        ));
        if let Some(project_layer) = self.project_layer.clone() {
            builder = builder.add_source(config::File::from_str(
                &project_layer.data.to_string(),
                config::FileFormat::Toml,
            ));
        }

        let _cfg: ComanConfig = builder.build()?.try_deserialize().wrap_err("invalid config")?;
        Ok(())
    }
}

pub fn global_config_layer() -> Result<Layer> {
    let config_dir = get_config_dir();
    let source = config_dir.join(CONFIG_FILE_NAME.clone());
    Layer::from_path(source)
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

pub fn project_local_config_layer() -> Result<Option<Layer>> {
    if let Some(source) = get_project_local_config_file() {
        let layer = Layer::from_path(source)?;
        return Ok(Some(layer));
    }
    Ok(None)
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
#[cfg(test)]
mod tests {
    use claim::*;
    use current_dir::*;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_get_project_local_config() {
        let temp_dir = tempdir().expect("couldn't create temp dir");

        let pwd = temp_dir.path().join("sub").join("folder");
        std::fs::create_dir_all(pwd.clone()).expect("couldn't create dir");
        let mut locked_cwd = Cwd::mutex().lock().expect("couldn't get cwd lock");
        locked_cwd.set(&pwd).expect("couldn't set current dir");
        assert_eq!(pwd, std::env::current_dir().expect("couldn't get current dir"));

        let config = temp_dir.path().join("coman.toml");
        std::fs::write(&config, "").expect("couldn't create config file");

        let config_file = get_project_local_config_file();
        assert_some!(config_file.clone());
        assert_eq!(config_file.unwrap(), config);
    }

    #[test]
    fn test_layer_from_path() {
        let temp_dir = tempdir().expect("couldn't create temp dir");
        let config = temp_dir.path().join("coman.toml");
        let content = "#some comment[cscs]\nvalue=10\n";
        std::fs::write(&config, content).expect("couldn't write config file");
        let layer = Layer::from_path(config.clone()).expect("couldn't load config");
        assert_eq!(layer.source, config);
        assert_eq!(layer.data.to_string(), content);
    }

    #[test]
    fn test_layer_get_set() {
        let temp_dir = tempdir().expect("couldn't create temp dir");
        let config = temp_dir.path().join("coman.toml");
        let content = "[cscs]\nvalue=10\n";
        std::fs::write(&config, content).expect("couldn't write config file");
        let mut layer = Layer::from_path(config.clone()).expect("couldn't load config");
        assert_eq!(layer.get("cscs.value").unwrap().unwrap(), "10");
        assert_none!(layer.get("cscs.other_value").unwrap());
        layer.set("cscs.other_value", 20).unwrap();
        assert_eq!(layer.get("cscs.other_value").unwrap().unwrap(), "20");
        assert_eq!(layer.data.to_string(), "[cscs]\nvalue=10\nother_value = 20\n");
    }

    #[test]
    fn test_config_read_write() {
        let project_dir = tempdir().expect("couldn't create temp dir");

        let mut locked_cwd = Cwd::mutex().lock().expect("couldn't get cwd lock");
        locked_cwd.set(&project_dir).expect("couldn't set current dir");
        assert_eq!(
            project_dir.path(),
            std::env::current_dir().expect("couldn't get current dir")
        );

        let project_config = project_dir.path().join("coman.toml");
        std::fs::write(&project_config, "[cscs]\ncurrent_system = \"project\"").expect("couldn't create config file");

        let home_dir = tempdir().expect("couldn't create temp dir");
        let global_config = home_dir.path().join(".config").join("coman").join("coman.toml");
        println!(
            "global: {}, project: {}",
            global_config.display(),
            project_config.display()
        );
        std::fs::create_dir_all(global_config.parent().unwrap()).expect("couldn't create config dir");
        std::fs::write(&global_config, "[cscs]\ncurrent_system = \"global\"").expect("couldn't create config file");

        let default_layer: DocumentMut = DEFAULT_CONFIG_TOML.parse().expect("couldn't parse default config");
        let global_layer = Layer::from_path(global_config).expect("couldn't create global layer");
        let project_layer = Layer::from_path(project_config).expect("couldn't create project layer");
        let mut builder =
            config::Config::builder().add_source(config::File::from_str(DEFAULT_CONFIG_TOML, config::FileFormat::Toml));
        builder = builder
            .add_source(config::File::from_str(
                &global_layer.data.to_string(),
                config::FileFormat::Toml,
            ))
            .add_source(config::File::from_str(
                &project_layer.data.to_string(),
                config::FileFormat::Toml,
            ));

        let cfg: ComanConfig = builder
            .build()
            .expect("couldn't build")
            .try_deserialize()
            .expect("couldn't deserialize");
        let mut conf = Config {
            values: cfg,
            default_layer,
            global_layer,
            project_layer: Some(project_layer),
        };

        assert_eq!(conf.values.cscs.current_system, "project");
        conf.set("cscs.current_system", "global2", true)
            .expect("couldn't set global config value");
        assert_eq!(conf.values.cscs.current_system, "project");
        conf.set("cscs.current_system", "project2", false)
            .expect("couldn't set global config value");
        assert_eq!(conf.values.cscs.current_system, "project2");
    }
}
