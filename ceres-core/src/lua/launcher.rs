use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread;

use path_absolutize::Absolutize;
use rlua::prelude::*;

use crate::error::*;
use crate::evloop::{get_event_loop_tx, Message};
use crate::lua::util::wrap_result;

pub struct LaunchConfig {
    launch_command: String,
    path_prefix:    Option<String>,
    extra_args:     Vec<String>,
}

fn run_map(map_path: &str, config: LaunchConfig) -> Result<(), anyhow::Error> {
    let map_path: PathBuf = map_path.into();
    let map_path = map_path.absolutize()?;
    let map_path = map_path
        .to_str()
        .ok_message("path to map must be valid UTF-8")?;
    let map_path = config.path_prefix.map_or_else(
        || Cow::Borrowed(map_path),
        |prefix| Cow::Owned(prefix + map_path),
    );

    let mut cmd = Command::new(config.launch_command);

    let log_file = fs::File::create("war3.log").context("could not create wc3 log file")?;
    cmd.arg("-loadfile")
        .arg(map_path.as_ref())
        .arg("-launch")
        .stdout(
            log_file
                .try_clone()
                .context("could not clone log file handle to stdout")?,
        )
        .stderr(
            log_file
                .try_clone()
                .context("could not clone log file handle to stderr")?,
        );

    for arg in config.extra_args {
        cmd.arg(arg);
    }

    let tx = get_event_loop_tx();

    thread::spawn(move || {
        println!("Starting Warcraft III with command line:\n{:?}", cmd);
        let child = cmd.spawn().context("could not launch Warcraft III");

        match child {
            Ok(mut child) => {
                if let Err(error) = child.wait() {
                    println!("Process terminated errorfully: {}", error)
                }
            }
            Err(error) => println!("An error occured while starting WC3: {}", error),
        }

        tx.send(Message::ChildTerminated)
    });

    Ok(())
}

fn lua_run_map(path: LuaString, config: LuaTable) -> Result<bool, anyhow::Error> {
    let map_path = path.to_str()?;

    let launch_command: String = config
        .get("command")
        .context("could not read 'command' field")?;
    let path_prefix: Option<String> = config
        .get("prefix")
        .context("could not read 'prefix' field")?;
    let args: Option<Vec<String>> = config.get("args").context("could not read 'args' field")?;

    let config = LaunchConfig {
        launch_command,
        path_prefix,
        extra_args: args.unwrap_or_default(),
    };

    run_map(map_path, config)?;

    Ok(true)
}

pub fn get_runmap_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx, (path, config): (LuaString, LuaTable)| {
        let result = lua_run_map(path, config);

        Ok(wrap_result(ctx, result))
    })
    .unwrap()
}
