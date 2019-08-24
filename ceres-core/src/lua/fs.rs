use std::path::PathBuf;
use std::fs;

use rlua::prelude::*;
use err_derive::Error;
use path_absolutize::Absolutize;

use crate::error::AnyError;
use crate::error::IoError;
use crate::error::ContextError;

#[derive(Error, Debug)]
pub enum LuaFileError {
    #[error(display = "Path validation attempt failed: {}", cause)]
    PathCanonicalizationFailed { cause: IoError },
    #[error(display = "Invalid path")]
    InvalidPath,
}

impl From<LuaFileError> for LuaError {
    fn from(err: LuaFileError) -> LuaError {
        LuaError::external(err)
    }
}

fn validate_path(path: &str) -> Result<PathBuf, LuaFileError> {
    let path = PathBuf::from(&path);

    path.absolutize().map_err(|err| {
        dbg!(LuaFileError::PathCanonicalizationFailed {
            cause: IoError::new(path, err),
        })
    })
}

fn lua_write_file(path: &str, content: LuaString) -> Result<(), AnyError> {
    let path = validate_path(&path)?;

    fs::create_dir_all(path.parent().ok_or(LuaFileError::InvalidPath)?)?;
    fs::write(path, content.as_bytes())?;

    Ok(())
}

fn lua_read_file<'lua>(ctx: LuaContext<'lua>, path: &str) -> Result<LuaString<'lua>, AnyError> {
    let path = validate_path(&path)?;

    let content = fs::read(path)?;

    Ok(ctx.create_string(&content).unwrap())
}

pub fn get_fs_module(ctx: LuaContext) -> LuaTable {
    let table = ctx.create_table().unwrap();

    {
        let func = ctx
            .create_function(
                move |ctx: LuaContext, (path, content): (String, LuaString)| {
                    let result = lua_write_file(&path, content);

                    match result {
                        Ok(()) => Ok((true, LuaValue::Nil)),
                        Err(err) => Ok((
                            false,
                            LuaValue::String(ctx.create_string(&err.to_string())?),
                        )),
                    }
                },
            )
            .unwrap();

        table.set("writeFile", func).unwrap();
    }

    {
        let func = ctx
            .create_function(move |ctx: LuaContext, path: (String)| {
                let result = lua_read_file(ctx, &path);

                match result {
                    Ok(s) => Ok((LuaValue::String(s), LuaValue::Nil)),
                    Err(err) => Ok((
                        LuaValue::Nil,
                        LuaValue::String(ctx.create_string(&err.to_string())?),
                    )),
                }
            })
            .unwrap();

        table.set("readFile", func).unwrap();
    }

    table
}
