pub mod map;
pub mod util;
pub mod compiler;
pub mod macros;
pub mod fs;

use std::path::PathBuf;

use rlua::prelude::*;

use crate::CeresRunMode;

pub fn setup_ceres_environ(
    ctx: LuaContext,
    base_path: PathBuf,
    run_mode: CeresRunMode,
    manifest_requested: bool,
    script_args: Vec<String>,
) {
    const CERES_BUILDSCRIPT_LIB: &str = include_str!("../resource/ceres_buildscript_lib.lua");

    let globals = ctx.globals();

    let ceres_table = ctx.create_table().unwrap();

    ceres_table
        .set("registerMacro", macros::get_register_luafn(ctx))
        .unwrap();
    ceres_table
        .set("compileScript", compiler::get_compile_script_luafn(ctx))
        .unwrap();
    ceres_table
        .set("loadMap", map::get_map_load_luafn(ctx))
        .unwrap();

    ceres_table
        .set(
            "isRunmapRequested",
            ctx.create_function::<_, bool, _>(move |_, _: ()| match run_mode {
                CeresRunMode::RunMap => Ok(true),
                _ => Ok(false),
            })
            .unwrap(),
        )
        .unwrap();

    ceres_table
        .set(
            "isManifestRequested",
            ctx.create_function::<_, bool, _>(move |_, _: ()| Ok(manifest_requested))
                .unwrap(),
        )
        .unwrap();

    ceres_table
        .set(
            "sendManifest",
            ctx.create_function::<_, (), _>(|_, _: ()| Ok(())).unwrap(),
        )
        .unwrap();

    let fs_table = fs::get_fs_module(ctx, base_path);

    globals.set("fs", fs_table).unwrap();
    globals.set("ceres", ceres_table).unwrap();

    ctx.load(CERES_BUILDSCRIPT_LIB).exec().unwrap();
}
