use std::fs;
use std::io::{Read, BufReader, SeekFrom, Seek};

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

impl LuaUserData for Viewer {
    fn add_methods<'lua, T>(methods: &mut T)
    where
        T: LuaUserDataMethods<'lua, Self>,
    {
        methods.add_method_mut("readFile", |ctx, obj, path: (LuaString)| {
            let result = lua_mpq_readfile(&mut obj.archive, path);

            Ok(lua_wrap_result(ctx, result))
        });
        methods.add_method_mut("list", |ctx, obj, _: ()| Ok(obj.archive.files()));

        methods.add_method_mut("header", |ctx, obj, _: ()| {
            let result = lua_mpq_getheader(&mut obj.archive);

            Ok(lua_wrap_result(ctx, result))
        });
    }
}

fn lua_mpq_readfile(archive: &mut FileArchive, path: LuaString) -> Result<Vec<u8>, AnyError> {
    let path = path.to_str()?;
    Ok(archive.read_file(path)?)
}

fn lua_mpq_getheader(archive: &mut FileArchive) -> Result<Option<Vec<u8>>, AnyError> {
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

fn lua_open_mpq(path: &str) -> Result<Viewer, AnyError> {
    let file = fs::OpenOptions::new().read(true).open(path)?;

    let file = BufReader::new(file);
    let archive = Archive::open(file)?;

    Ok(Viewer { archive })
}

fn get_mpqopen_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx: LuaContext, path: (String)| {
        let result = lua_open_mpq(&path);

        Ok(lua_wrap_result(ctx, result))
    })
    .unwrap()
}

pub fn get_mpq_module(ctx: LuaContext) -> LuaTable {
    let table = ctx.create_table().unwrap();

    table.set("open", get_mpqopen_luafn(ctx)).unwrap();

    table
}
