use clap::clap_app;
use fern::colors::{Color, ColoredLevelConfig};
use log::error;

use failure::{Error, ResultExt};


fn main() -> Result<(), Box<std::error::Error>> {
    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .warn(Color::Yellow)
        .error(Color::Red);

    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{}] -> {}",
                colors.color(record.level()),
                message
            ))
        })
        .chain(std::io::stdout())
        .apply()?;

    let matches = clap_app!(Ceres =>
        (version: "0.1.2")
        (author: "mori <mori@reu.moe>")
        (about: "Ceres is a build tool, script compiler and map preprocessor for WC3 Lua maps.")
        (@subcommand build =>
            (about: "Builds the specified map")
            (@arg MAPDIR: +required "Specifies the mapdir to use for the build.")
        )
        (@subcommand run =>
            (about: "Builds and runs the specified map")
            (@arg MAPDIR: +required "Specifies the mapdir to use for the run-build.")
        )
        (@subcommand parse => (@arg FILE: +required "Debug."))
    )
    .get_matches();

    std::process::exit(match run(matches) {
        Err(error) => {
            error!("{}", error);

            for (i, cause) in error.iter_causes().enumerate() {
                error!("{}Cause: {}", "    ".repeat(i + 1), cause);
            }

            1
        }
        Ok(_) => 0,
    });
}

fn run(matches: clap::ArgMatches) -> Result<(), Error> {
    if let Some(arg) = matches.subcommand_matches("build") {
        let mut ceres = ceres_core::Ceres::new()?;
        ceres
            .build_map(arg.value_of("MAPDIR").unwrap())
            .context("Could not build map.")?;
    } else if let Some(arg) = matches.subcommand_matches("run") {
        let mut ceres = ceres_core::Ceres::new()?;
        ceres
            .run_map(arg.value_of("MAPDIR").unwrap())
            .context("Could not run map.")?;
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
    }

    Ok(())
}