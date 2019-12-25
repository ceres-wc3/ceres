use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use clap::clap_app;
use anyhow::Result;

use ceres_formats::*;
use ceres_formats::objectstore::ObjectStore;

fn main() {
    dotenv::dotenv().ok();

    let matches = clap_app!(Util =>
        (version: "0.0.0")
        (author: "mori <mori@reu.moe>")
        (about: "Utilities")
        (@subcommand dump =>
            (about: "Dump info")
            (@arg type: --type +takes_value)
            (@arg format: --format -f +takes_value)
        )
        (@subcommand parseobj =>
            (about: "Parse obj file and dump info")
            (@arg type: --type +takes_value)
            (@arg format: --format -f +takes_value)
            (@arg FILE: +required +takes_value)
        )
        (@subcommand rwobj =>
            (about: "Parse and write obj file")
            (@arg FILE: +required +takes_value)
        )
        (@subcommand dbg =>
            (about: "dbg")
        )
    )
    .get_matches();

    std::process::exit(match run(matches) {
        Err(error) => {
            println!("[ERROR] An error has occured. Error chain:");
            println!("{:?}", error);

            1
        }
        Ok(_) => 0,
    });
}

fn serialize_obj<O: Serialize + std::fmt::Debug>(object: O, out_format: &str) {
    match out_format {
        "dbg" => {
            println!("{:#?}", object);
        }
        "bin" => {
            bincode::serialize_into(std::io::stdout().lock(), &object).unwrap();
        }
        "ron" => {
            let pretty_config = ron::ser::PrettyConfig {
                depth_limit:            10,
                new_line:               "\n".into(),
                indentor:               "    ".into(),
                separate_tuple_members: false,
                enumerate_arrays:       false,
            };

            println!(
                "{}",
                ron::ser::to_string_pretty(&object, pretty_config).unwrap()
            );
        }
        _ => {
            panic!("wow");
        }
    }
}

fn run(matches: clap::ArgMatches) -> Result<()> {
    let data_dir = std::env::var("DATA_DIR")?;

    let meta = metadata::read_metadata_dir(&data_dir);
    eprintln!("loaded metadata information");
    let data = objectstore::read_data_dir(&data_dir, &meta);
    eprintln!("loaded data information");

    if let Some(arg) = matches.subcommand_matches("dump") {
        let dump_type = arg.value_of("type").unwrap_or_else(|| "all");
        let format = arg.value_of("format").unwrap_or_else(|| "dbg");

        match dump_type {
            "all" => serialize_obj(&(&meta, &data), format),
            "meta" => serialize_obj(&meta, format),
            "obj" => serialize_obj(&data, format),
            _ => panic!("hello"),
        }
    } else if let Some(arg) = matches.subcommand_matches("parseobj") {
        let file_path: PathBuf = arg.value_of("FILE").unwrap().into();
        let format = arg.value_of("format").unwrap_or_else(|| "dbg");
        let kind = file_path.extension().unwrap().to_string_lossy();
        let mut data = ObjectStore::default();

        parser::w3obj::read::read_object_file(
            &fs::read(&file_path)?,
            &mut data,
            ObjectKind::from_ext(&kind),
        )?;

        serialize_obj(&data, format);
    } else if let Some(_arg) = matches.subcommand_matches("dbg") {
        dbg!(meta.field_by_id(ObjectId::from_bytes(b"amac").unwrap()));
    } else if let Some(arg) = matches.subcommand_matches("rwobj") {
        let file_path: PathBuf = arg.value_of("FILE").unwrap().into();
        let kind = file_path.extension().unwrap().to_string_lossy();
        let mut data = ObjectStore::default();

        parser::w3obj::read::read_object_file(
            &fs::read(&file_path)?,
            &mut data,
            ObjectKind::from_ext(&kind),
        )?;

        parser::w3obj::write::write_object_file(
            std::io::stdout(),
            &meta,
            &data,
            ObjectKind::from_ext(&kind),
        )?;
    }

    Ok(())
}
