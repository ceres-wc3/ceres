#![allow(dead_code)]

extern crate ceres_mpq as mpq;

pub(crate) mod lua;
pub(crate) mod error;
pub(crate) mod compiler;

use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::borrow::Cow;

use rlua::prelude::*;

use crate::error::AnyError;
use crate::error::CompilerError;

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

pub fn execute_script(
    run_mode: CeresRunMode,
    script_args: Vec<&str>,
    manifest_port: Option<u16>,
    script: &str,
) -> Result<(), AnyError> {
    const DEFAULT_BUILD_SCRIPT: &str = include_str!("resource/buildscript_default.lua");

    let lua = Rc::new(Lua::new());

    let result: Result<(), LuaError> = lua.context(|ctx| {
        // scoped so that we don't have to synchronize anything...
        ctx.scope(|_| {
            lua::setup_ceres_environ(
                ctx,
                run_mode,
                manifest_port.is_some(),
                script_args.into_iter().map(|s| s.into()).collect(),
            );

            ctx.load(script).exec()?;

            Ok(())
        })
    });

    if let Err(LuaError::CallbackError { cause, .. }) = &result {
        if let LuaError::ExternalError(err) = cause.as_ref() {
            if let Some(err) = err.downcast_ref::<CompilerError>() {
                println!("[ERROR] A compiler error occured:\n{}", err);
            } else {
                println!("[ERROR] An unknown error in Ceres occured:\n{:?}", err);
            }
        } else {
            println!("[ERROR] An unknown error in Ceres occured:\n{:?}", cause);
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
    manifest_port: Option<u16>,
) -> Result<(), AnyError> {
    const DEFAULT_BUILD_SCRIPT: &str = include_str!("resource/buildscript_default.lua");

    let build_script_path = project_dir.join("build.lua");
    let build_script = fs::read_to_string(&build_script_path)
        .ok()
        .map(Cow::Owned)
        .unwrap_or_else(|| Cow::Borrowed(DEFAULT_BUILD_SCRIPT));

    execute_script(run_mode, script_args, manifest_port, build_script.as_ref())
}
