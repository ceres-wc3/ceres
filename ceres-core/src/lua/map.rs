use crate::error::IoError;

use rlua::prelude::*;
use err_derive::*;

use std::path::{PathBuf, Path};
use std::fs;

pub enum MapMode {
    File,
    Dir,
}

#[derive(Error, Debug)]
pub enum MapError {
    #[error(display = "Could not read map: {}", cause)]
    MapUnreadable { cause: IoError },
    #[error(display = "Not implemented: {}", message)]
    NotImplemented { message: String },
    #[error(display = "Not a map")]
    NotAMap,
}

impl MapError {
    fn not_implemented(message: String) -> MapError {
        MapError::NotImplemented { message }
    }
}

impl From<IoError> for MapError {
    fn from(err: IoError) -> MapError {
        MapError::MapUnreadable { cause: err }
    }
}

impl From<MapError> for LuaError {
    fn from(err: MapError) -> LuaError {
        LuaError::external(err)
    }
}

pub struct Map {
    path: PathBuf,
    mode: MapMode,
}

impl Map {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Map, MapError> {
        let path = path.as_ref();

        // check that the file is readable
        fs::metadata(&path).map_err(|err| IoError::new(path, err))?;

        let mode = if path.is_file() {
            return Err(MapError::not_implemented(
                "Loading a map from a file".into(),
            ));
        } else if path.is_dir() {
            MapMode::Dir
        } else {
            return Err(MapError::NotAMap);
        };

        Ok(Map {
            path: path.into(),
            mode,
        })
    }

    pub fn save_to_folder<P: AsRef<Path>>(path: P) -> Result<(), IoError> {
        

        unimplemented!()
    }

    fn validate(&self) -> Result<(), String> {
        match self.mode {
            MapMode::Dir => Ok(()),
            MapMode::File => Err("Map files are unsupported at this moment.".into()),
        }
    }
}

impl LuaUserData for Map {
    fn add_methods<'lua, T>(methods: &mut T)
    where
        T: LuaUserDataMethods<'lua, Self>,
    {
        methods.add_method("isValid", |ctx, obj, _: ()| {
            let result = obj.validate();

            if let Err(err) = result {
                let err_msg = ctx.create_string(&err).unwrap();
                return Ok((false, LuaValue::String(err_msg)));
            }

            Ok((true, LuaValue::Nil))
        })
    }
}

pub fn get_map_load_luafn(ctx: LuaContext) -> LuaFunction {
    let func = ctx.create_function(|ctx, path: (String)| {
        let map = Map::open(path)?;

        Ok(ctx.create_userdata(map).unwrap())
    });

    func.unwrap()
}
