use crate::error::IoError;

use rlua::prelude::*;
use err_derive::*;
use walkdir::WalkDir;

use std::iter::Iterator;
use std::path::{PathBuf, Path};
use std::fs;
use std::collections::HashMap;

use ceres_mpq as mpq;

const BLANK_MAP_FILE: &[u8] = include_bytes!("../resource/blank.w3x");

#[derive(Error, Debug)]
pub enum MapError {
    #[error(display = "{}", cause)]
    Io { cause: IoError },
    #[error(display = "Not implemented: {}", message)]
    NotImplemented { message: String },
    #[error(display = "MPQ Operation Failed: {}", cause)]
    MpqError { cause: mpq::MpqError },
    #[error(display = "Map file path is malformed")]
    MalformedFilePath,
}

impl MapError {
    fn not_implemented(message: String) -> MapError {
        MapError::NotImplemented { message }
    }
}

impl From<IoError> for MapError {
    fn from(err: IoError) -> MapError {
        MapError::Io { cause: err }
    }
}

impl From<MapError> for LuaError {
    fn from(err: MapError) -> LuaError {
        LuaError::external(err)
    }
}

impl From<mpq::MpqError> for MapError {
    fn from(err: mpq::MpqError) -> MapError {
        MapError::MpqError { cause: err }
    }
}

pub struct DirMap {
    root:  PathBuf,
    cache: HashMap<mpq::MPQPath, Vec<u8>>,
}

impl DirMap {
    pub fn new<P: Into<PathBuf>>(path: P) -> DirMap {
        DirMap {
            root:  path.into(),
            cache: HashMap::default(),
        }
    }

    fn read_file(&self, path: &str) -> Option<Vec<u8>> {
        let mpq_path = mpq::MPQPath::from_buf(&path)?;

        if self.cache.contains_key(&mpq_path) {
            return self.cache.get(&mpq_path).map(|s| s.clone());
        }

        fs::read(self.root.join(path)).ok()
    }

    fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), MapError> {
        let mpq_path = mpq::MPQPath::from_buf(path).ok_or(MapError::MalformedFilePath)?;

        self.cache.insert(mpq_path, data.into());
        Ok(())
    }

    fn save_to_mpq<P: AsRef<Path>>(&self, path: P) -> Result<(), MapError> {
        let path = path.as_ref().to_str().unwrap();
        fs::write(path, BLANK_MAP_FILE).map_err(|err| IoError::new(path, err))?;
        let mpq_archive = mpq::MPQArchive::open(path).unwrap();

        for (path, data) in &self.cache {
            mpq_archive.write_file(path, data)?;
        }

        for file_name in self.get_files() {
            let mpq_path = mpq::MPQPath::from_buf(&file_name).ok_or(MapError::MalformedFilePath)?;

            if !self.cache.contains_key(&mpq_path) {
                let data = fs::read(self.root.join(&file_name)).unwrap();
                mpq_archive.write_file(&mpq_path, data).unwrap()
            }
        }

        Ok(())
    }

    fn save_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), MapError> {
        for entry in WalkDir::new(&self.root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|d| d.file_type().is_file())
        {
            let relative_path = entry.path().strip_prefix(&self.root).unwrap();
            fs::copy(entry.path(), path.as_ref().join(relative_path))
                .map_err(|err| IoError::new(&path, err))?;
        }

        Ok(())
    }

    fn get_files(&self) -> Vec<String> {
        WalkDir::new(&self.root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| {
                e.path()
                    .strip_prefix(&self.root)
                    .map(|s| s.to_str().map(|s| s.to_string()))
                    .ok()
            })
            .filter_map(|e| e)
            .collect()
    }

    fn validate(&self) -> Result<(), MapError> {
        Ok(())
    }
}

impl LuaUserData for DirMap {
    fn add_methods<'lua, T>(methods: &mut T)
    where
        T: LuaUserDataMethods<'lua, Self>,
    {
        methods.add_method("isValid", |ctx, obj, _: ()| {
            let result = obj.validate();

            if let Err(err) = result {
                let err_msg = ctx.create_string(&err.to_string()).unwrap();
                return Ok((false, LuaValue::String(err_msg)));
            }

            Ok((true, LuaValue::Nil))
        })
    }
}

pub struct MpqMap {
    path: PathBuf,
    archive: mpq::MPQArchive,
    cache:   HashMap<mpq::MPQPath, Vec<u8>>,
}

impl MpqMap {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<MpqMap, MapError> {
        let archive = mpq::MPQArchive::open(&path)?;

        Ok(MpqMap {
            path: path.as_ref().into(),
            archive,
            cache: Default::default(),
        })
    }

    fn read_file(&self, path: &str) -> Option<Vec<u8>> {
        let mpq_path = mpq::MPQPath::from_buf(&path)?;

        if self.cache.contains_key(&mpq_path) {
            self.cache.get(&mpq_path).map(|s| s.clone())
        } else {
            let file = self.archive.open_file(&mpq_path).ok()?;
            let data = file.read_contents().ok()?;

            Some(data)
        }
    }

    fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), MapError> {
        let mpq_path = mpq::MPQPath::from_buf(path).ok_or(MapError::MalformedFilePath)?;

        self.cache.insert(mpq_path, data.into());
        Ok(())
    }

    fn save_to_mpq<P: AsRef<Path>>(&self, path: P) -> Result<(), MapError> {
        fs::copy(&self.path, &path).map_err(|err| IoError::new(&path, err))?;
        let mpq_archive = mpq::MPQArchive::open(&path)?;

        for (path, data) in &self.cache {
            mpq_archive.write_file(path, data)?;
        }

        Ok(())
    }

    fn save_to_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), MapError> {
        unimplemented!()
    }

    fn get_files(&self) -> Vec<String> {
        unimplemented!()
    }

    fn validate(&self) -> Result<(), MapError> {
        unimplemented!()
    }
}

fn load_map_internal(
    ctx: LuaContext,
    path: String,
) -> Result<Result<LuaAnyUserData, String>, LuaError> {
    let path: PathBuf = path.into();

    if !path.exists() {
        return Ok(Err(format!("Map not found: {:?}", path)));
    }

    if path.is_dir() {
        let map = DirMap::new(path);

        let user_data = ctx.create_userdata(map)?;
        return Ok(Ok(user_data));
    }

    if path.is_file() {
        unimplemented!()
    }

    Ok(Err("What the fuck?".into()))
}

pub fn get_map_load_luafn(ctx: LuaContext) -> LuaFunction {
    let func = ctx.create_function(|ctx, path: (String)| {
        let result = load_map_internal(ctx, path)?;

        match result {
            Err(msg) => Ok((
                LuaValue::Boolean(false),
                LuaValue::String(ctx.create_string(&msg)?),
            )),
            Ok(user_data) => Ok((LuaValue::UserData(user_data), LuaValue::Nil)),
        }
    });

    func.unwrap()
}
