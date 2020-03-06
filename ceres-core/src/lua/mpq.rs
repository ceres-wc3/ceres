use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

use mpq::Archive;
use mpq::Creator;
use mpq::FileOptions;
use rlua::prelude::*;
use walkdir::WalkDir;

use crate::error::ContextError;
use crate::error::StringError;
use crate::lua::util::wrap_result;

type FileArchive = Archive<BufReader<fs::File>>;

struct Viewer {
    archive: FileArchive,
}

struct Builder {
    creator: Creator,
}

impl LuaUserData for Viewer {
    fn add_methods<'lua, T>(methods: &mut T)
    where
        T: LuaUserDataMethods<'lua, Self>,
    {
        methods.add_method_mut("readFile", |ctx, obj, path: LuaString| {
            let result =
                readflow_readfile(&mut obj.archive, path).map(|s| ctx.create_string(&s).unwrap());

            Ok(wrap_result(ctx, result))
        });

        methods.add_method_mut("files", |_, obj, _: ()| {
            if let Some(files) = obj.archive.files() {
                return Ok(Some(
                    files
                        .iter()
                        .map(|p| p.replace("\\", "/"))
                        .collect::<Vec<_>>(),
                ));
            }

            Ok(None)
        });

        methods.add_method_mut("extractTo", |ctx, obj, path: LuaString| {
            let result = readflow_extract(&mut obj.archive, path);

            Ok(wrap_result(ctx, result))
        });
    }
}

impl LuaUserData for Builder {
    fn add_methods<'lua, T>(methods: &mut T)
    where
        T: LuaUserDataMethods<'lua, Self>,
    {
        methods.add_method_mut(
            "add",
            |ctx, obj, (path, contents, options): (LuaString, LuaString, Option<LuaTable>)| {
                let options = fileoptions_from_table(options);
                let result = writeflow_addbuf(obj, path, contents, options);

                Ok(wrap_result(ctx, result))
            },
        );

        methods.add_method_mut(
            "addFromFile",
            |ctx, obj, (archive_path, fs_path, options): (LuaString, LuaString, Option<LuaTable>)| {
                let options = fileoptions_from_table(options);
                let result = writeflow_addfile(obj, archive_path, fs_path, options);

                Ok(wrap_result(ctx, result))
            },
        );

        methods.add_method_mut(
            "addFromDir",
            |ctx, obj, (dir_path, options): (LuaString, Option<LuaTable>)| {
                let options = fileoptions_from_table(options);
                let result = writeflow_adddir(obj, dir_path, options);

                Ok(wrap_result(ctx, result))
            },
        );

        methods.add_method_mut(
            "addFromMpq",
            |ctx, obj, (viewer, options): (LuaAnyUserData, Option<LuaTable>)| {
                let mut viewer = viewer.borrow_mut::<Viewer>()?;
                let options = fileoptions_from_table(options);
                let result = writeflow_addmpq(obj, &mut viewer.archive, options);

                Ok(wrap_result(ctx, result))
            },
        );

        methods.add_method_mut("write", |ctx, obj, path: LuaString| {
            let result = writeflow_write(obj, path);

            Ok(wrap_result(ctx, result))
        });
    }
}

fn fileoptions_from_table(table: Option<LuaTable>) -> FileOptions {
    if let Some(table) = table {
        let mut options = FileOptions::default();

        if let Ok(Some(encrypt)) = table.get::<_, Option<bool>>("encrypt") {
            options.encrypt = encrypt;
        }

        if let Ok(Some(compress)) = table.get::<_, Option<bool>>("compress") {
            options.compress = compress;
        }

        options
    } else {
        let mut options = FileOptions::default();
        options.encrypt = false;
        options.compress = true;

        options
    }
}

fn readflow_readfile(archive: &mut FileArchive, path: LuaString) -> Result<Vec<u8>, anyhow::Error> {
    let path = path.to_str()?;
    Ok(archive.read_file(path)?)
}

fn readflow_extract(archive: &mut FileArchive, path: LuaString) -> Result<bool, anyhow::Error> {
    let path: PathBuf = path.to_str()?.into();
    fs::create_dir_all(path.parent().unwrap())
        .map_err(|cause| ContextError::new("could not create folder for map", cause))?;

    let files = archive
        .files()
        .ok_or_else(|| StringError::new("no listfile found"))?;
    for file_path in files {
        let contents = archive.read_file(&file_path);

        if let Err(error) = contents {
            eprintln!("mpq.extractTo(): could not read file {}: {}", file_path, error);
            continue;
        }

        let file_path = file_path.replace("\\", "/");
        let out_path = path.join(&file_path);

        if let Err(error) = fs::create_dir_all(out_path.parent().unwrap()) {
            eprintln!(
                "mpq.extractTo(): could not create directory for file {}: {}",
                file_path, error
            );
            continue;
        }

        if let Err(error) = fs::write(out_path, contents.unwrap()) {
            eprintln!("mpq.extractTo(): could not write file {}: {}", file_path, error);
            continue;
        }
    }

    Ok(true)
}

fn readflow_open(path: &str) -> Result<Viewer, anyhow::Error> {
    let file = fs::OpenOptions::new().read(true).open(path)?;

    let file = BufReader::new(file);
    let archive = Archive::open(file)?;

    Ok(Viewer { archive })
}

fn writeflow_addbuf(
    builder: &mut Builder,
    path: LuaString,
    contents: LuaString,
    options: FileOptions,
) -> Result<bool, anyhow::Error> {
    let path = path.to_str()?;
    let contents = contents.as_bytes();

    builder.creator.add_file(path, contents, options);

    Ok(true)
}

fn writeflow_addfile(
    builder: &mut Builder,
    archive_path: LuaString,
    fs_path: LuaString,
    options: FileOptions,
) -> Result<bool, anyhow::Error> {
    let archive_path = archive_path.to_str()?;
    let fs_path = fs_path.to_str()?;
    let contents = fs::read(fs_path)?;
    builder.creator.add_file(archive_path, contents, options);

    Ok(true)
}

fn writeflow_adddir(
    builder: &mut Builder,
    dir_path: LuaString,
    options: FileOptions,
) -> Result<bool, anyhow::Error> {
    let dir_path: PathBuf = dir_path.to_str()?.into();

    let entries = WalkDir::new(&dir_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|s| s.ok())
        .filter(|s| s.file_type().is_file());

    for entry in entries {
        let contents = fs::read(entry.path());

        if let Err(error) = contents {
            eprintln!(
                "mpq.addFromDir(): could not add file {}: {}",
                entry.path().display(),
                error
            );
            continue;
        }

        let relative_path = entry.path().strip_prefix(&dir_path).unwrap();
        builder
            .creator
            .add_file(relative_path.to_str().unwrap(), contents.unwrap(), options);
    }

    Ok(true)
}

fn writeflow_addmpq(
    builder: &mut Builder,
    archive: &mut FileArchive,
    options: FileOptions,
) -> Result<bool, anyhow::Error> {
    let files = archive
        .files()
        .ok_or_else(|| StringError::new("no listfile found"))?;

    for file in files {
        let contents = archive.read_file(&file);

        if let Err(error) = contents {
            eprintln!("mpq.addFromMpq(): couldn't add file {}: {}", &file, error);
            continue;
        }

        builder.creator.add_file(&file, contents.unwrap(), options);
    }

    Ok(true)
}

fn writeflow_write(builder: &mut Builder, path: LuaString) -> Result<bool, anyhow::Error> {
    let path: PathBuf = path.to_str()?.into();

    fs::create_dir_all(path.parent().unwrap())
        .map_err(|cause| ContextError::new("could not create folder for map", cause))?;

    let writer = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)?;

    let mut writer = BufWriter::new(writer);
    let creator = &mut builder.creator;
    creator.write(&mut writer)?;

    Ok(true)
}

fn writeflow_new() -> Result<Builder, anyhow::Error> {
    let creator = Creator::default();

    Ok(Builder { creator })
}

fn get_mpqopen_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx: LuaContext, path: String| {
        let result = readflow_open(&path);

        Ok(wrap_result(ctx, result))
    })
    .unwrap()
}

fn get_mpqnew_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx: LuaContext, _: ()| {
        let result = writeflow_new();

        Ok(wrap_result(ctx, result))
    })
    .unwrap()
}

pub fn get_mpq_module(ctx: LuaContext) -> LuaTable {
    let table = ctx.create_table().unwrap();

    table.set("open", get_mpqopen_luafn(ctx)).unwrap();
    table.set("create", get_mpqnew_luafn(ctx)).unwrap();

    table
}
