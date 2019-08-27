use std::fs;
use std::io::{Read, Write, BufReader, BufWriter, SeekFrom, Seek};

use rlua::prelude::*;
use mpq::Archive;
use mpq::Creator;
use mpq::FileOptions;

use crate::error::AnyError;
use crate::error::IoError;
use crate::lua::util::lua_wrap_result;

type FileArchive = Archive<BufReader<fs::File>>;

struct Viewer {
    archive: FileArchive,
}

struct Builder {
    creator: Creator,
    writer:  BufWriter<fs::File>,
}

impl LuaUserData for Viewer {
    fn add_methods<'lua, T>(methods: &mut T)
    where
        T: LuaUserDataMethods<'lua, Self>,
    {
        methods.add_method_mut("readFile", |ctx, obj, path: (LuaString)| {
            let result = readflow_readfile(&mut obj.archive, path);

            Ok(lua_wrap_result(ctx, result))
        });

        methods.add_method_mut("files", |_, obj, _: ()| Ok(obj.archive.files()));

        methods.add_method_mut("header", |ctx, obj, _: ()| {
            let result = readflow_getheader(&mut obj.archive);

            Ok(lua_wrap_result(ctx, result))
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

                Ok(lua_wrap_result(ctx, result))
            },
        );

        methods.add_method_mut(
            "addFromFile",
            |ctx, obj, (archive_path, fs_path, options): (LuaString, LuaString, Option<LuaTable>)| {
                let options = fileoptions_from_table(options);
                let result = writeflow_addfile(obj, archive_path, fs_path, options);

                Ok(lua_wrap_result(ctx, result))
            },
        );
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

fn readflow_readfile(archive: &mut FileArchive, path: LuaString) -> Result<Vec<u8>, AnyError> {
    let path = path.to_str()?;
    Ok(archive.read_file(path)?)
}

fn readflow_getheader(archive: &mut FileArchive) -> Result<Option<Vec<u8>>, AnyError> {
    let start = archive.start();

    if start > 0 {
        let reader = archive.reader();
        reader.seek(SeekFrom::Start(0))?;
        let mut buf = vec![0u8; start as usize];
        reader.read_exact(&mut buf)?;
        Ok(Some(buf))
    } else {
        Ok(None)
    }
}

fn readflow_open(path: &str) -> Result<Viewer, AnyError> {
    let file = fs::OpenOptions::new().read(true).open(path)?;

    let file = BufReader::new(file);
    let archive = Archive::open(file)?;

    Ok(Viewer { archive })
}

fn writeflow_setheader(builder: &mut Builder, contents: &[u8]) -> Result<(), AnyError> {
    let writer = &mut builder.writer;
    writer.seek(SeekFrom::Start(0))?;
    writer.write_all(contents)?;

    Ok(())
}

fn writeflow_addbuf(
    builder: &mut Builder,
    path: LuaString,
    contents: LuaString,
    options: FileOptions,
) -> Result<bool, AnyError> {
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
) -> Result<bool, AnyError> {
    let archive_path = archive_path.to_str()?;
    let fs_path = fs_path.to_str()?;
    let contents = fs::read(fs_path)?;
    builder.creator.add_file(archive_path, contents, options);

    Ok(true)
}

fn writeflow_new(path: &str) -> Result<Builder, AnyError> {
    let writer = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)?;

    let writer = BufWriter::new(writer);
    let creator = Creator::default();

    Ok(Builder { creator, writer })
}

fn get_mpqopen_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx: LuaContext, path: (String)| {
        let result = readflow_open(&path);

        Ok(lua_wrap_result(ctx, result))
    })
    .unwrap()
}

fn get_mpqnew_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx: LuaContext, path: (String)| {
        let result = writeflow_new(&path);

        Ok(lua_wrap_result(ctx, result))
    })
    .unwrap()
}

pub fn get_mpq_module(ctx: LuaContext) -> LuaTable {
    let table = ctx.create_table().unwrap();

    table.set("open", get_mpqopen_luafn(ctx)).unwrap();

    table
}
