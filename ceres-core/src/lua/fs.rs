use rlua::prelude::*;
use err_derive::Error;
use path_absolutize::Absolutize;

use std::path::PathBuf;

use crate::error::IoError;
use crate::error::ContextError;

#[derive(Error, Debug)]
pub enum PathValidationError {
    #[error(
        display = "Cannot read outside base directory! {:?} is not located inside {:?}",
        attempted,
        base_dir
    )]
    OutsideBaseDirectory {
        attempted: PathBuf,
        base_dir:  PathBuf,
    },
    #[error(display = "Path validation attempt failed: {}", cause)]
    PathCanonicalizationFailed { cause: IoError },
}

impl From<PathValidationError> for LuaError {
    fn from(err: PathValidationError) -> LuaError {
        LuaError::external(err)
    }
}

fn verify_base_path(path: &PathBuf, base_path: &PathBuf) -> Result<(), PathValidationError> {
    if !path.starts_with(&base_path) {
        return Err(PathValidationError::OutsideBaseDirectory {
            attempted: path.into(),
            base_dir:  base_path.into(),
        });
    }

    Ok(())
}

fn validate_path(path: &str) -> Result<PathBuf, PathValidationError> {
    let path = PathBuf::from(&path);

    path.absolutize().map_err(|err| {
        dbg!(PathValidationError::PathCanonicalizationFailed {
            cause: IoError::new(path, err),
        })
    })
}

pub fn get_fs_module(ctx: LuaContext, base_path: PathBuf) -> LuaTable {
    use std::fs;

    let table = ctx.create_table().unwrap();

    {
        let base_path = base_path.clone().canonicalize().unwrap();

        let func = ctx
            .create_function(
                move |ctx: LuaContext, (path, content): (String, LuaString)| {
                    let result = validate_path(&path)
                        .map_err(|err| err.to_string())
                        .and_then(|path| {
                            verify_base_path(&path, &base_path).map_err(|err| err.to_string())?;
                            Ok(path)
                        })
                        .and_then(|path| {
                            fs::create_dir_all(path.parent().unwrap())
                                .and_then(|()| fs::write(&path, content.as_bytes()))
                                .map_err(|err| {
                                    ContextError::new(
                                        "Failed to write file",
                                        IoError::new(path, err),
                                    )
                                })
                                .map_err(|err| err.to_string())
                        });

                    match result {
                        Ok(()) => return Ok((true, LuaValue::Nil)),
                        Err(err) => return Ok((false, LuaValue::String(ctx.create_string(&err)?))),
                    }
                },
            )
            .unwrap();

        table.set("writeFile", func).unwrap();
    }

    {
        let base_path = base_path.clone().canonicalize().unwrap();

        let func = ctx
            .create_function(move |ctx: LuaContext, path: (String)| {
                let result = validate_path(&path)
                    .map_err(|err| err.to_string())
                    .and_then(|path| {
                        verify_base_path(&path, &base_path).map_err(|err| err.to_string())?;
                        Ok(path)
                    })
                    .and_then(|path| {
                        fs::read(&path)
                            .map(|s| ctx.create_string(&s).unwrap())
                            .map_err(|err| {
                                ContextError::new("Failed to read file", IoError::new(path, err))
                            })
                            .map_err(|err| err.to_string())
                    });

                match result {
                    Ok(s) => Ok((LuaValue::String(s), LuaValue::Nil)),
                    Err(err) => Ok((LuaValue::Nil, LuaValue::String(ctx.create_string(&err)?))),
                }
            })
            .unwrap();

        table.set("readFile", func).unwrap();
    }

    table
}
