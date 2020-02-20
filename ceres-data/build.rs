use std::env;
use std::fs;
use std::path::PathBuf;

use ceres_formats::metadata;
use ceres_formats::objectstore;

fn main() {
    let out_dir: PathBuf = env::var("OUT_DIR").unwrap().into();
    let crate_dir: PathBuf = env::var("CARGO_MANIFEST_DIR").unwrap().into();

    let meta = metadata::read_metadata_dir(crate_dir.join("data"));
    let data = objectstore::read_data_dir("data", &meta);
    let data = objectstore::ObjectStoreStock::new(&data);

    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(out_dir.join("data.bin"))
        .unwrap();

    bincode::serialize_into(&mut file, &(&data, &meta)).unwrap();
}
