use clap::clap_app;

use failure::{Error};

fn main() -> Result<(), Box<std::error::Error>> {
    let matches = clap_app!(Ceres =>
        (version: "0.1.2")
        (author: "mori <mori@reu.moe>")
        (about: "Ceres is a build tool, script compiler and map preprocessor for WC3 Lua maps.")
        (@subcommand build =>
            (about: "Uses the build.lua file in the current directory to build a map.")
            (setting: clap::AppSettings::TrailingVarArg)
            (@arg dir: --dir -d +takes_value "Sets the project directory.")
            (@arg BUILD_ARGS: ... "Arguments to pass to the build script.")
        )
        (@subcommand run =>
            (about: "Uses the build.lua file in the current directory to build and run a map.")
            (setting: clap::AppSettings::TrailingVarArg)
            (@arg dir: --dir -d +takes_value "Sets the project directory.")
            (@arg BUILD_ARGS: ... "Arguments to pass to the build script.")
        )
        (@subcommand parse => (@arg FILE: +required "Debug."))
        (@subcommand mpqtest =>
            (@arg FILE: +required)
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

fn run_build(arg: &clap::ArgMatches, mode: ceres_core::CeresRunMode) -> Result<(), Error> {
    let project_dir = arg
        .value_of("dir")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    let script_args = arg
        .values_of("BUILD_ARGS")
        .map(std::iter::Iterator::collect)
        .unwrap_or_else(Vec::new);

    ceres_core::execute(mode, project_dir, script_args)?;

    Ok(())
}

fn run(matches: clap::ArgMatches) -> Result<(), Error> {
    if let Some(arg) = matches.subcommand_matches("build") {
        run_build(arg, ceres_core::CeresRunMode::Build)?;
    } else if let Some(arg) = matches.subcommand_matches("run") {
        run_build(arg, ceres_core::CeresRunMode::RunMap)?;
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
    } else if let Some(arg) = matches.subcommand_matches("mpqtest") {
        use ceres_mpq as mpq;

        let filename = arg.value_of("FILE").unwrap();
        dbg!(filename);

        let archive = mpq::MPQArchive::open(filename)?;
        let file = archive.open_file("(listfile)")?;
        println!("{}", String::from_utf8_lossy(&file.read_contents()?))
    }

    Ok(())
}
