#![allow(dead_code)]

extern crate ceres_data as w3data;
extern crate ceres_mpq as mpq;

use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use rlua::prelude::*;

use crate::error::ContextError;
use crate::evloop::wait_on_evloop;

pub(crate) mod lua;
pub(crate) mod error;
pub(crate) mod compiler;
pub(crate) mod evloop;

#[derive(Copy, Clone)]
pub enum CeresRunMode {
    Build,
    RunMap,
    LiveReload,
}

pub fn lua_error_root_cause(error: &LuaError) -> anyhow::Error {
    match error {
        LuaError::CallbackError { traceback, cause } => {
            anyhow::anyhow!("{}\n{}", lua_error_root_cause(cause), traceback)
        }
        LuaError::ExternalError(external) => {
            if let Some(error) = Error::downcast_ref::<LuaError>(external.as_ref()) {
                lua_error_root_cause(error)
            } else {
                anyhow::anyhow!("{}", external)
            }
        }
        other => anyhow::anyhow!("{}", other),
    }
}

pub fn handle_lua_result(result: anyhow::Result<()>) {
    if let Err(err) = result {
        match err.downcast::<LuaError>() {
            Ok(err) => {
                println!("{}", lua_error_root_cause(&err));
            }
            Err(err) => println!("{}", err),
        }
    }
}

pub fn execute_script<F>(
    run_mode: CeresRunMode,
    script_args: Vec<&str>,
    action: F,
) -> Result<(), anyhow::Error>
where
    F: FnOnce(LuaContext) -> Result<(), anyhow::Error>,
{
    const DEFAULT_BUILD_SCRIPT: &str = include_str!("resource/buildscript_default.lua");

    let lua = Rc::new(Lua::new());

    let result: Result<(), anyhow::Error> = lua.context(|ctx| {
        lua::setup_ceres_environ(
            ctx,
            run_mode,
            script_args.into_iter().map(|s| s.into()).collect(),
        );

        action(ctx)?;

        Ok(())
    });

    if result.is_err() {
        handle_lua_result(result);
        std::process::exit(1);
    }

    wait_on_evloop(Rc::clone(&lua));

    Ok(())
}

pub fn run_build_script(
    run_mode: CeresRunMode,
    project_dir: PathBuf,
    script_args: Vec<&str>,
) -> Result<(), anyhow::Error> {
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

    execute_script(run_mode, script_args, |ctx| {
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
