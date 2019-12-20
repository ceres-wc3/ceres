#![allow(dead_code)]

extern crate ceres_mpq as mpq;
extern crate ceres_data as w3data;

pub(crate) mod lua;
pub(crate) mod error;
pub(crate) mod compiler;

use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use rlua::prelude::*;

use crate::error::AnyError;
use crate::error::CompilerError;
use crate::error::ContextError;

#[derive(Copy, Clone)]
pub enum CeresRunMode {
    Build,
    RunMap,
    LiveReload,
}

fn send_manifest_data(port: u16) {
    use std::net::TcpStream;

    use std::io::Write;

    let mut connection = TcpStream::connect(("localhost", port)).unwrap();

    println!("Woof woof");

    write!(connection, "Hello World!").unwrap();
}

pub fn execute_script<F>(
    run_mode: CeresRunMode,
    script_args: Vec<&str>,
    extension_port: Option<u16>,
    action: F,
) -> Result<(), AnyError>
where
    F: FnOnce(LuaContext) -> Result<(), AnyError>,
{
    const DEFAULT_BUILD_SCRIPT: &str = include_str!("resource/buildscript_default.lua");

    let lua = Rc::new(Lua::new());

    let result: Result<(), LuaError> = lua.context(|ctx| {
        lua::setup_ceres_environ(
            ctx,
            run_mode,
            script_args.into_iter().map(|s| s.into()).collect(),
            extension_port,
        );

        action(ctx).map_err(LuaError::external)?;

        Ok(())
    });

    if let Err(LuaError::ExternalError(cause)) = &result {
        if let Some(LuaError::CallbackError {traceback, cause}) = cause.downcast_ref::<LuaError>() {
            println!("[ERROR] An error occured while executing the script:");
            println!("{}", cause);
            println!("{}", traceback);
        } else {
            println!("[ERROR] Unknown error:");
            println!("[ERROR] {}", cause);
        }
    } else if let Err(err) = &result {
        println!("[ERROR] A Lua error occured in the build script:\n{}", err);
    }

    if result.is_err() {
        std::process::exit(1);
    }

    Ok(())
}

pub fn run_build_script(
    run_mode: CeresRunMode,
    project_dir: PathBuf,
    script_args: Vec<&str>,
    extension_port: Option<u16>,
) -> Result<(), AnyError> {
    const DEFAULT_BUILD_SCRIPT: &str = include_str!("resource/buildscript_default.lua");

    let build_script_path = project_dir.join("build.lua");

    let build_script = if build_script_path.is_file() {
        Some(
            fs::read_to_string(&build_script_path)
                .map_err(|cause| ContextError::new("Could not read custom build script", cause))?,
        )
    } else {
        None
    };

    execute_script(run_mode, script_args, extension_port, |ctx| {
        if let Some(build_script) = build_script {
            ctx.load(&build_script)
                .set_name("custom build script")
                .unwrap()
                .exec()?;
        }

        ctx.load(DEFAULT_BUILD_SCRIPT)
            .set_name("buildscript_default.lua")
            .unwrap()
            .exec()?;

        Ok(())
    })
}
