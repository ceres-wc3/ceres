pub mod util;
pub mod compiler;
pub mod macros;
pub mod fs;
pub mod mpq;
pub mod launcher;
pub mod object;

use std::net::TcpStream;
use std::io::Write;
use std::rc::Rc;

use rlua::prelude::*;
use rlua_serde::from_value;
use serde_json::to_string;
use serde::{Serialize, Deserialize};

use crate::CeresRunMode;

#[derive(Serialize, Deserialize)]
struct ProjectLayout {
    #[serde(rename = "mapsDirectory")]
    maps_directory: String,
    #[serde(rename = "srcDirectory")]
    src_directory: String,
    #[serde(rename = "libDirectory")]
    lib_directory: String,
    #[serde(rename = "targetDirectory")]
    target_directory: String,
}

fn send_layout(port: Option<u16>, layout: LuaTable) {
    if let Some(port) = port {
        let layout: ProjectLayout = from_value(LuaValue::Table(layout)).unwrap();
        let layout = to_string(&layout).unwrap();
        let mut connection = TcpStream::connect(("localhost", port)).unwrap();
        write!(connection, "{}", layout).unwrap();
    }
}

pub fn setup_ceres_environ(
    ctx: LuaContext,
    run_mode: CeresRunMode,
    script_args: Vec<String>,
    extension_port: Option<u16>,
) {
    const CERES_BUILDSCRIPT_LIB: &str = include_str!("../resource/buildscript_lib.lua");

    let globals = ctx.globals();

    let ceres_table = ctx.create_table().unwrap();

    ceres_table
        .set("registerMacro", macros::get_register_luafn(ctx))
        .unwrap();
    ceres_table
        .set("compileScript", compiler::get_compile_script_luafn(ctx))
        .unwrap();

    ceres_table
        .set(
            "runMode",
            ctx.create_function(move |ctx, _: ()| match run_mode {
                CeresRunMode::RunMap => Ok(ctx.create_string("run")),
                CeresRunMode::Build => Ok(ctx.create_string("build")),
                CeresRunMode::LiveReload => Ok(ctx.create_string("reload")),
            })
            .unwrap(),
        )
        .unwrap();

    ceres_table
        .set(
            "isLayoutRequested",
            ctx.create_function(move |_, _: ()| Ok(extension_port.is_some()))
                .unwrap(),
        )
        .unwrap();

    ceres_table
        .set(
            "sendLayout",
            ctx.create_function(move |_, layout: LuaTable| {
                send_layout(extension_port, layout);
                Ok(())
            })
            .unwrap(),
        )
        .unwrap();

    ceres_table
        .set(
            "getScriptArgs",
            ctx.create_function(move |_, _: ()| Ok(script_args.clone()))
                .unwrap(),
        )
        .unwrap();

    ceres_table
        .set("runWarcraft", launcher::get_runmap_luafn(ctx))
        .unwrap();

    ceres_table
        .set("loadObjects", object::get_open_store_from_str_luafn(ctx))
        .unwrap();



    let fs_table = fs::get_fs_module(ctx);
    let mpq_table = mpq::get_mpq_module(ctx);

    globals.set("fs", fs_table).unwrap();
    globals.set("mpq", mpq_table).unwrap();
    globals.set("ceres", ceres_table).unwrap();

    ctx.load(CERES_BUILDSCRIPT_LIB)
        .set_name("buildscript_lib.lua")
        .unwrap()
        .exec()
        .unwrap();
}
