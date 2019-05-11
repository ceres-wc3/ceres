use clap::clap_app;
use log::error;

use failure::{Error, ResultExt};


fn main() -> Result<(), Box<std::error::Error>> {
    let matches = clap_app!(Ceres =>
        (version: "0.1.2")
        (author: "mori <mori@reu.moe>")
        (about: "Ceres is a build tool, script compiler and map preprocessor for WC3 Lua maps.")
        (@subcommand build =>
            (about: "Uses the build.lua file in the current directory to build a map.")
            (setting: clap::AppSettings::TrailingVarArg)
            (@arg BUILD_ARGS: ... "Arguments to pass to the build script.")
        )
        (@subcommand run =>
            (about: "Uses the build.lua file in the current directory to build a map, and then runs it.")
            (setting: clap::AppSettings::TrailingVarArg)
            (@arg BUILD_ARGS: ... "Arguments to pass to the build script.")
        )
        (@subcommand parse => (@arg FILE: +required "Debug."))
        (@subcommand newbuild => 
            (about: "Uses a build.lua file to build a map.")
            (setting: clap::AppSettings::TrailingVarArg)
            (@arg dir: --dir -d +takes_value "Sets the project directory.")
            (@arg BUILD_ARGS: ... "Arguments to pass to the build script.")
        )
    )
    .get_matches();

    std::process::exit(match run(matches) {
        Err(error) => {
            println!("[ERROR] An error has occured. Error chain:");
            println!("{}", error);

            for cause in error.iter_causes() {
                println!("{}", cause);
            }

            1
        }
        Ok(_) => 0,
    });
}

fn run(matches: clap::ArgMatches) -> Result<(), Error> {
    if let Some(arg) = matches.subcommand_matches("build") {
        // let mut ceres = ceres_core::Ceres::new()?;
        // ceres
        //     .build_map(arg.value_of("MAPDIR").unwrap())
        //     .context("Could not build map.")?;
    } else if let Some(arg) = matches.subcommand_matches("run") {
        // let mut ceres = ceres_core::Ceres::new()?;
        // ceres
        //     .run_map(arg.value_of("MAPDIR").unwrap())
        //     .context("Could not run map.")?;
    } else if let Some(arg) = matches.subcommand_matches("parse") {
        // this is just some debugging ...

        use ceres_parsers::lua;
        use pest::Parser;

        let input = std::fs::read_to_string(arg.value_of("FILE").unwrap())?;

        let a = lua::LuaParser::parse(lua::Rule::Chunk, &input).unwrap();

        fn prnt(pairs: pest::iterators::Pairs<lua::Rule>, indent: usize) {
            for pair in pairs {
                println!(
                    "{}>{:?}: {}",
                    " ".repeat(indent),
                    pair.as_rule(),
                    pair.as_str().replace("\n", " ")
                );

                prnt(pair.into_inner(), indent + 1);
            }
        }

        prnt(a, 0);
    } else if let Some(arg) = matches.subcommand_matches("newbuild") {
        let script_args = arg
            .values_of("BUILD_ARGS")
            .map(std::iter::Iterator::collect)
            .unwrap_or_else(Vec::new);

        ceres_core::run_build_script(None, script_args)?;
    }

    Ok(())
}