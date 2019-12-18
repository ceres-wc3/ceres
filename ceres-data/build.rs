use std::env;
use std::path::PathBuf;
use std::fs;

use ceres_formats::metadata;
use ceres_formats::object;

fn main() {
    let out_dir: PathBuf = env::var("OUT_DIR").unwrap().into();
    let crate_dir: PathBuf = env::var("CARGO_MANIFEST_DIR").unwrap().into();

    let meta = metadata::read_metadata_dir(crate_dir.join("data"));
    let data = object::read_data_dir("data", &meta);
    let data = object::ObjectStoreStock::new(&data);

    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(out_dir.join("data.bin"))
        .unwrap();

    bincode::serialize_into(&mut file, &(&meta, &data)).unwrap();
}
