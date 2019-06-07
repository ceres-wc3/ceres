use rlua::prelude::*;
use err_derive::Error;

use std::path::PathBuf;

use crate::error::AnyError;
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

    path.canonicalize()
        .map_err(|err| PathValidationError::PathCanonicalizationFailed {
            cause: IoError::new(path, err),
        })
}

pub fn get_fs_module(ctx: LuaContext, base_path: PathBuf) -> LuaTable {
    use std::fs;

    let table = ctx.create_table().unwrap();

    {
        let base_path = base_path.clone().canonicalize().unwrap();

        let func = ctx
            .create_function(
                move |_ctx: LuaContext, (path, content): (String, LuaString)| {
                    let path = validate_path(&path)?;

                    verify_base_path(&path, &base_path)?;

                    fs::write(&path, content.as_bytes())
                        .map_err(|err| {
                            ContextError::new("Failed to write file", IoError::new(path, err))
                        })
                        .map_err(LuaError::external)?;

                    Ok(())
                },
            )
            .unwrap();

        table.set("writeFile", func).unwrap();
    }

    {
        let base_path = base_path.clone().canonicalize().unwrap();

        let func = ctx
            .create_function(move |ctx: LuaContext, path: (String)| {
                let path = validate_path(&path)?;

                verify_base_path(&path, &base_path)?;

                fs::read(&path)
                    .map(|s| ctx.create_string(&s).unwrap())
                    .map_err(|err| {
                        ContextError::new("Failed to read file", IoError::new(path, err))
                    })
                    .map_err(LuaError::external)
            })
            .unwrap();

        table.set("readFile", func).unwrap();
    }

    table
}
