#![allow(dead_code)]

pub mod lua;

// mod config;
pub mod error;
mod compiler;

use rlua::prelude::*;

use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

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

pub fn run_build_script(
    run_mode: CeresRunMode,
    project_dir: PathBuf,
    script_args: Vec<&str>,
    manifest_port: Option<u16>,
) -> Result<(), AnyError> {
    const DEFAULT_BUILD_SCRIPT: &str = include_str!("resource/ceres_buildscript_default.lua");

    let build_script_path = project_dir.join("build.lua");
    let build_script = fs::read_to_string(&build_script_path).ok();

    let lua = Rc::new(Lua::new());

    let result: Result<(), LuaError> = lua.context(|ctx| {
        // scoped so that we don't have to synchronize anything...
        ctx.scope(|_| {
            lua::setup_ceres_environ(
                ctx,
                project_dir,
                run_mode,
                manifest_port.is_some(),
                script_args.into_iter().map(|s| s.into()).collect(),
            );

            let build_script_src = if build_script.is_some() {
                build_script.as_ref().unwrap()
            } else {
                DEFAULT_BUILD_SCRIPT
            };

            ctx.load(build_script_src).exec()?;

            Ok(())
        })
    });

    use std::any::Any;
    if let Err(err) = &result {
        println!("{:?}", err.type_id());
    }

    if let Err(LuaError::CallbackError{..}) = &result {
        println!("wtf");
    }

    if let Err(LuaError::ExternalError(err)) = result {
        if let Some(err) = err.downcast_ref::<CompilerError>() {
            println!("[ERROR] A compiler error occured:\n{}", err);
            std::process::exit(1);
        }
    } else if let Err(err) = result {
        println!("[ERROR] A Lua error occured in the build script:\n{}", err);
        std::process::exit(1);
    }

    Ok(())
}
