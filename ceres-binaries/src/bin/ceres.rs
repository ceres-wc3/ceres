use std::error::Error;

use clap::clap_app;

type AnyError = Box<dyn Error + Sync + Send + 'static>;

fn main() {
    let matches = clap_app!(Ceres =>
        (version: "0.2.0")
        (author: "mori <mori@reu.moe>")
        (about: "Ceres is a build tool, script compiler and map preprocessor for WC3 Lua maps.")
        (@subcommand build =>
            (about: "Uses the build.lua file in the current directory to build a map.")
            (setting: clap::AppSettings::TrailingVarArg)
            (@arg manifest: --manifest +takes_value)
            (@arg dir: --dir -d +takes_value "Sets the project directory.")
            (@arg BUILD_ARGS: ... "Arguments to pass to the build script.")
        )
        (@subcommand run =>
            (about: "Uses the build.lua file in the current directory to build and run a map.")
            (setting: clap::AppSettings::TrailingVarArg)
            (@arg manifest: --manifest +takes_value)
            (@arg dir: --dir -d +takes_value "Sets the project directory.")
            (@arg BUILD_ARGS: ... "Arguments to pass to the build script.")
        )
        (@subcommand exec =>
            (about: "Executes the specified lua file using Ceres runtime")
            (setting: clap::AppSettings::TrailingVarArg)
            (@arg script: +required +takes_value)
            (@arg BUILD_ARGS: ... "Arguments to pass to the build script.")
        )
    )
    .get_matches();

    std::process::exit(match run(matches) {
        Err(error) => {
            println!("[ERROR] An error has occured. Error chain:");
            println!("{}", error);

            let mut cause = error.source();
            while let Some(inner_cause) = cause {
                println!("{}", &inner_cause);
                cause = inner_cause.source();
            }

            1
        }
        Ok(_) => 0,
    });
}

fn run_build(arg: &clap::ArgMatches, mode: ceres_core::CeresRunMode) -> Result<(), AnyError> {
    let project_dir = arg
        .value_of("dir")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    let script_args = arg
        .values_of("BUILD_ARGS")
        .map(std::iter::Iterator::collect)
        .unwrap_or_else(Vec::new);

    let manifest_port = arg
        .value_of("manifest")
        .map(|s| u16::from_str_radix(s, 10).unwrap());

    ceres_core::run_build_script(mode, project_dir, script_args, manifest_port)?;

    Ok(())
}

fn exec(arg: &clap::ArgMatches) -> Result<(), AnyError> {
    let script = arg
        .value_of("script")
        .map(std::path::PathBuf::from)
        .unwrap();

    let script = std::fs::read_to_string(script)?;

    ceres_core::execute_script(ceres_core::CeresRunMode::Build, Vec::new(), None, |ctx| {
        ctx.load(&script).exec()?;
        
        Ok(())
    })?;

    Ok(())
}

fn run(matches: clap::ArgMatches) -> Result<(), AnyError> {
    if let Some(arg) = matches.subcommand_matches("build") {
        run_build(arg, ceres_core::CeresRunMode::Build)?;
    } else if let Some(arg) = matches.subcommand_matches("run") {
        run_build(arg, ceres_core::CeresRunMode::RunMap)?;
    } else if let Some(arg) = matches.subcommand_matches("exec") {
        exec(arg)?;
    }

    Ok(())
}
