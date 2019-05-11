use std::fs;
use std::path::PathBuf;

use failure::Fail;
use serde::Deserialize;

#[derive(Fail, Debug)]
pub enum CeresContextError {
    #[fail(display = "Could not initialize Ceres config.")]
    CouldNotReadConfig(#[fail(cause)] ConfigError),
    #[fail(display = "Source file not found ({:?})", _0)]
    SourceFileNotFound(String),
    #[fail(display = "Map ({:?}) not found", _0)]
    MapNotFound(String),
    #[fail(display = "Map ({:?}) file not found ({:?})", _0, _1)]
    MapFileNotFound(String, String),
}

pub struct CeresPaths {
    root: PathBuf,
    src: PathBuf,
    lib: PathBuf,
    target: PathBuf,
    maps: PathBuf
}

pub struct CeresContext {
    config: CeresConfig,
    root_dir: PathBuf,
}

impl CeresContext {
    pub fn new<P: Into<PathBuf>>(root_dir: P) -> Result<CeresContext, CeresContextError> {
        let config =
            CeresConfig::initialize().map_err(CeresContextError::CouldNotReadConfig)?;

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

#[derive(Fail, Debug)]
pub enum ConfigError {
    #[fail(display = "User's home directory was not found")]
    HomeFolderNotFound,
    #[fail(display = "No config could be loaded.")]
    NoConfigLoaded,
    #[fail(display = "Could not find config ({:?})", _0)]
    ConfigReadError(PathBuf, #[fail(cause)] std::io::Error),
    #[fail(display = "Config is malformed ({:?})", _0)]
    ConfigMalformed(PathBuf, #[fail(cause)] toml::de::Error),
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
