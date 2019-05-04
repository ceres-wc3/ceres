use std::fs;
use std::path::{PathBuf, Path};
use std::collections::HashMap;

use dirs;
use failure::{err_msg, format_err, Error, Fail, ResultExt};
use log::info;
use serde::Deserialize;

#[derive(Fail, Debug)]
pub enum CeresContextError {
    #[fail(display = "No config could be loaded.")]
    NoConfigLoaded,
    #[fail(display = "User's home directory was not found")]
    HomeFolderNotFound,
    #[fail(display = "Could not find Ceres config ({:?})", _0)]
    ConfigReadError(PathBuf, #[fail(cause)] std::io::Error),
    #[fail(display = "Ceres config is malformed ({:?})", _0)]
    ConfigMalformed(PathBuf, #[fail(cause)] toml::de::Error),
    #[fail(display = "Source file not found ({:?})", _0)]
    SourceFileNotFound(String),
    #[fail(display = "Map ({:?}) not found", _0)]
    MapNotFound(String),
    #[fail(display = "Map ({:?}) file not found ({:?})", _0, _1)]
    MapFileNotFound(String, String),
}

pub struct CeresContext {
    config: CeresConfig,
    root_dir: PathBuf,
}

impl CeresContext {
    pub fn new<P: Into<PathBuf>>(root_dir: P) -> Result<CeresContext, CeresContextError> {
        let config = CeresConfig::initialize()?;

        Ok(CeresContext {
            config,
            root_dir: root_dir.into(),
        })
    }

    pub fn config(&self) -> &CeresConfig {
        &self.config
    }

    pub fn root_dir_path(&self) -> PathBuf {
        self.root_dir.clone()
    }

    pub fn target_dir_path(&self) -> PathBuf {
        self.root_dir.join("target")
    }

    pub fn file_path(&self, file_name: &str) -> PathBuf {
        self.root_dir.join(file_name)
    }

    pub fn src_file_path<F: AsRef<str>>(&self, file_name: F) -> Result<PathBuf, CeresContextError> {
        // try ./lib folder first
        let lib_path = self.root_dir.join("lib").join(file_name.as_ref());
        if lib_path.is_file() {
            return Ok(lib_path);
        }

        // try ./src next
        let src_path = self.root_dir.join("src").join(file_name.as_ref());
        if src_path.is_file() {
            return Ok(src_path);
        }

        Err(CeresContextError::SourceFileNotFound(
            file_name.as_ref().to_string(),
        ))
    }

    pub fn map_src_dir_path(&self, map_name: &str) -> Result<PathBuf, CeresContextError> {
        let path = self.root_dir.join("maps").join(map_name);

        if path.is_dir() {
            Ok(path)
        } else {
            Err(CeresContextError::MapNotFound(map_name.to_string()))
        }
    }

    pub fn map_file_path(
        &self,
        map_name: &str,
        file_name: &str,
    ) -> Result<PathBuf, CeresContextError> {
        let map_dir = self.map_src_dir_path(map_name)?;
        let file_path = map_dir.join(file_name);

        if file_path.is_file() {
            Ok(file_path)
        } else {
            Err(CeresContextError::MapFileNotFound(
                map_name.to_string(),
                file_name.to_string(),
            ))
        }
    }

    pub fn map_target_dir_path(&self, map_name: &str) -> PathBuf {
        let path = self.root_dir.join("target").join(map_name);

        if path.is_file() {
            fs::remove_file(&path).unwrap();
        }

        if !path.is_dir() {
            fs::create_dir_all(&path).unwrap();
        }

        path
    }
}

#[derive(Deserialize)]
pub struct CeresConfig {
    pub run: CeresRunConfig,
    pub reload: Option<CeresReloadConfig>,
}

#[derive(Deserialize)]
pub struct CeresRunConfig {
    pub wc3_start_command: String,
    pub is_wine: Option<bool>,
    pub wine_disk_prefix: Option<String>,
    pub window_mode: Option<String>,
}

#[derive(Deserialize)]
pub struct CeresReloadConfig {
    pub wc3_docs_dir: String,
}

impl CeresConfig {
    fn initialize() -> Result<CeresConfig, CeresContextError> {
        let user_config = Self::load_user_config();
        let map_config = Self::load_map_config();

        match (user_config, map_config) {
            (Err(_), Err(_)) => {
                Err(CeresContextError::NoConfigLoaded)
            }

            (Ok(val), Err(_)) => {
                Ok(val.try_into().map_err(|_| CeresContextError::NoConfigLoaded)?)
            }

            (Err(_), Ok(val)) => {
                Ok(val.try_into().map_err(|_| CeresContextError::NoConfigLoaded)?)
            }

            (Ok(user), Ok(map)) => {
                // let user = user.try_into().map_err(|err| CeresContextError::NoConfigLoaded)?;
                // let map = map.try_into().map_err(|err| CeresContextError::NoConfigLoaded)?;

                let mut result = toml::map::Map::default();

                for (key, val) in user.as_table().unwrap().iter() {
                    result.insert(key.to_string(), val.clone());
                }

                for (key, val) in map.as_table().unwrap().iter() {
                    result.insert(key.to_string(), val.clone());
                }

                Ok(toml::Value::Table(result).try_into().map_err(|_| CeresContextError::NoConfigLoaded)?)
            }

            _ => unimplemented!()
        }
    }

    fn load_user_config() -> Result<toml::Value, CeresContextError> {
        let user_config_dir =
            dirs::home_dir().ok_or_else(|| CeresContextError::HomeFolderNotFound)?;
        let ceres_config_path = user_config_dir.join(".ceres").join("config.toml");

        let ceres_config_content = fs::read(&ceres_config_path)
            .map_err(|err| CeresContextError::ConfigReadError(ceres_config_path.clone(), err))?;

        let ceres_config = toml::from_slice(&ceres_config_content)
            .map_err(|err| CeresContextError::ConfigMalformed(ceres_config_path, err))?;

        Ok(ceres_config)
    }

    fn load_map_config() -> Result<toml::Value, CeresContextError> {
        let ceres_config_path = std::env::current_dir().unwrap().join("ceres.toml");

        let ceres_config_content = fs::read(&ceres_config_path)
            .map_err(|err| CeresContextError::ConfigReadError(ceres_config_path.clone(), err))?;

        let ceres_config = toml::from_slice(&ceres_config_content)
            .map_err(|err| CeresContextError::ConfigMalformed(ceres_config_path, err))?;

        Ok(ceres_config)
    }
}
