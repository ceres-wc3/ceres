use std::path::PathBuf;
use std::fs;
use std::sync::mpsc;
use std::time::Duration;

use notify::{Watcher, RecursiveMode, DebouncedEvent, watcher};
use rlua::prelude::*;
use err_derive::Error;
use path_absolutize::Absolutize;
use walkdir::WalkDir;

use crate::error::AnyError;
use crate::error::IoError;
use crate::lua::util::wrap_result;

#[derive(Error, Debug)]
pub enum LuaFileError {
    #[error(display = "Path validation attempt failed: {}", cause)]
    PathCanonizationFailed { cause: IoError },
    #[error(display = "Invalid path")]
    InvalidPath,
    #[error(display = "Not a directory")]
    NotADir,
}

impl From<LuaFileError> for LuaError {
    fn from(err: LuaFileError) -> LuaError {
        LuaError::external(err)
    }
}

fn validate_path(path: &str) -> Result<PathBuf, LuaFileError> {
    let path = PathBuf::from(&path);

    path.absolutize()
        .map_err(|err| LuaFileError::PathCanonizationFailed {
            cause: IoError::new(path, err),
        })
}

fn lua_write_file(path: &str, content: LuaString) -> Result<(), AnyError> {
    let path = validate_path(&path)?;

    fs::create_dir_all(path.parent().ok_or(LuaFileError::InvalidPath)?)?;
    fs::write(path, content.as_bytes())?;

    Ok(())
}

fn lua_copy_file(from: &str, to: &str) -> Result<(), AnyError> {
    let from = validate_path(&from)?;
    let to = validate_path(to)?;

    fs::create_dir_all(to.parent().ok_or(LuaFileError::InvalidPath)?)?;
    fs::copy(from, to)?;

    Ok(())
}

fn lua_read_file<'lua>(ctx: LuaContext<'lua>, path: &str) -> Result<LuaString<'lua>, AnyError> {
    let path = validate_path(&path)?;

    let content = fs::read(path)?;

    Ok(ctx.create_string(&content).unwrap())
}

fn lua_read_dir<'lua>(
    ctx: LuaContext<'lua>,
    path: &str,
) -> Result<(LuaTable<'lua>, LuaTable<'lua>), AnyError> {
    let path = validate_path(&path)?;

    if !path.is_dir() {
        return Err(Box::new(LuaFileError::NotADir));
    }

    let entries: Vec<_> = fs::read_dir(path)?.collect();

    let files = entries
        .iter()
        .filter_map(|s| s.as_ref().ok())
        .filter(|s| s.file_type().unwrap().is_file())
        .filter_map(|s| s.path().to_str().map(|s| s.to_string()))
        .map(|s| ctx.create_string(&s).unwrap());

    let dirs = entries
        .iter()
        .filter_map(|s| s.as_ref().ok())
        .filter(|s| s.file_type().unwrap().is_dir())
        .filter_map(|s| s.path().to_str().map(|s| s.to_string()))
        .map(|s| ctx.create_string(&s).unwrap());

    Ok((
        ctx.create_sequence_from(files).unwrap(),
        ctx.create_sequence_from(dirs).unwrap(),
    ))
}

fn lua_copy_dir(from: &str, to: &str) -> Result<bool, AnyError> {
    let from: PathBuf = from.into();
    let to: PathBuf = to.into();

    let entries = WalkDir::new(&from)
        .follow_links(true)
        .into_iter()
        .filter_map(|s| s.ok())
        .filter(|s| s.file_type().is_file());

    for entry in entries {
        let relative_path = entry.path().strip_prefix(&from).unwrap();
        let from = entry.path();
        let to = to.join(relative_path);

        if let Err(error) = fs::create_dir_all(to.parent().unwrap()) {
            eprintln!(
                "fs.copyDir(): error creating folder for {}: {}",
                to.display(),
                error
            );
        } else if let Err(error) = fs::copy(&from, &to) {
            eprintln!(
                "fs.copyDir(): error copying [{} -> {}]: {}",
                from.display(),
                to.display(),
                error
            );
        };
    }

    Ok(true)
}

fn lua_absolutize_path(path: &str) -> Result<String, AnyError> {
    let path: PathBuf = path.into();

    // TODO: Handle invalid UTF-8
    Ok(path.absolutize()?.to_str().unwrap().into())
}

fn lua_watch_file<'lua>(
    ctx: LuaContext<'lua>,
    path: &str,
    callback: LuaFunction<'lua>,
) -> Result<(), AnyError> {
    let (tx, rx) = mpsc::channel();
    let mut watcher = watcher(tx, Duration::from_millis(100))?;

    fs::write(path, "")?;
    watcher.watch(path, RecursiveMode::NonRecursive)?;

    loop {
        match rx.recv() {
            Ok(event) => match event {
                DebouncedEvent::Write(path) | DebouncedEvent::Create(path) => {
                    let data = fs::read(path)?;
                    callback.call::<_, ()>(LuaValue::String(ctx.create_string(&data)?))?;
                }
                _ => {}
            },
            Err(err) => {
                eprintln!("Error while watching file: {}", err);
                break;
            }
        }
    }

    Ok(())
}

fn get_writefile_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(move |ctx, (path, content): (String, LuaString)| {
        let result = lua_write_file(&path, content).map(|_| true);

        Ok(wrap_result(ctx, result))
    })
    .unwrap()
}

fn get_copyfile_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(move |ctx, (from, to): (String, String)| {
        let result = lua_copy_file(&from, &to);

        Ok(wrap_result(ctx, result))
    })
    .unwrap()
}

fn get_readfile_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx, path: String| {
        let result = lua_read_file(ctx, &path);

        Ok(wrap_result(ctx, result))
    })
    .unwrap()
}

fn get_readdir_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx, path: String| {
        let result = lua_read_dir(ctx, &path);

        Ok(wrap_result(ctx, result))
    })
    .unwrap()
}

fn get_isdir_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|_, path: String| {
        let path: PathBuf = path.into();

        Ok(path.is_dir())
    })
    .unwrap()
}

fn get_isfile_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|_, path: String| {
        let path: PathBuf = path.into();

        Ok(path.is_file())
    })
    .unwrap()
}

fn get_exists_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|_, path: String| {
        let path: PathBuf = path.into();

        Ok(path.exists())
    })
    .unwrap()
}

fn get_absolutize_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx, path: String| {
        let result = lua_absolutize_path(&path);

        Ok(wrap_result(ctx, result))
    })
    .unwrap()
}

fn get_copydir_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx, (from, to): (String, String)| {
        let result = lua_copy_dir(&from, &to);

        Ok(wrap_result(ctx, result))
    })
    .unwrap()
}

fn get_filewatch_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx, (target, callback): (String, LuaFunction)| {
        let result = lua_watch_file(ctx, &target, callback);

        Ok(wrap_result(ctx, result))
    })
    .unwrap()
}

pub fn get_fs_module(ctx: LuaContext) -> LuaTable {
    let table = ctx.create_table().unwrap();

    table.set("writeFile", get_writefile_luafn(ctx)).unwrap();
    table.set("copyFile", get_copyfile_luafn(ctx)).unwrap();
    table.set("readFile", get_readfile_luafn(ctx)).unwrap();
    table.set("readDir", get_readdir_luafn(ctx)).unwrap();
    table.set("isDir", get_isdir_luafn(ctx)).unwrap();
    table.set("isFile", get_isfile_luafn(ctx)).unwrap();
    table.set("exists", get_exists_luafn(ctx)).unwrap();
    table.set("absolutize", get_absolutize_luafn(ctx)).unwrap();
    table.set("copyDir", get_copydir_luafn(ctx)).unwrap();
    table.set("watchFile", get_filewatch_luafn(ctx)).unwrap();

    table
}
