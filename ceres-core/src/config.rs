use std::fs;
use std::path::PathBuf;

use failure::Fail;
use serde::Deserialize;

#[derive(Fail, Debug)]
pub enum ConfigError {
    #[fail(display = "User's home directory was not found")]
    HomeFolderNotFound,
    #[fail(display = "Could not find config ({:?})", _0)]
    ConfigReadError(PathBuf, #[fail(cause)] std::io::Error),
    #[fail(display = "Config is malformed ({:?})", _0)]
    ConfigMalformed(PathBuf, #[fail(cause)] toml::de::Error),
}

#[derive(Deserialize)]
pub struct CeresConfig {
    pub run:    Option<CeresRunConfig>,
    pub reload: Option<CeresReloadConfig>,
}

#[derive(Deserialize)]
pub struct CeresRunConfig {
    pub wc3_start_command: String,
    pub is_wine:           Option<bool>,
    pub wine_disk_prefix:  Option<String>,
    pub window_mode:       Option<String>,
}

#[derive(Deserialize)]
pub struct CeresReloadConfig {
    pub wc3_docs_dir: String,
}

impl CeresConfig {
    fn initialize() -> Result<CeresConfig, ConfigError> {
        Self::load_map_config()
    }

    fn load_map_config() -> Result<CeresConfig, ConfigError> {
        let ceres_config_path = std::env::current_dir().unwrap().join("ceres.toml");

        let ceres_config_content = fs::read(&ceres_config_path)
            .map_err(|err| ConfigError::ConfigReadError(ceres_config_path.clone(), err))?;

        let ceres_config = toml::from_slice(&ceres_config_content)
            .map_err(|err| ConfigError::ConfigMalformed(ceres_config_path, err))?;

        Ok(ceres_config)
    }
}
