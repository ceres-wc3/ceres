use lazy_static::lazy_static;
use ceres_formats::object::ObjectStoreStatic;
use ceres_formats::metadata::MetadataStore;

const BUNDLED_DATA_BIN: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/data.bin"));

lazy_static! {
    static ref BUNDLED_DATA: (ObjectStoreStatic, MetadataStore) = {
        let data: (ObjectStoreStatic, MetadataStore) = bincode::deserialize(BUNDLED_DATA_BIN).unwrap();

        data
    };
}

pub fn metadata() -> &'static MetadataStore {
    &BUNDLED_DATA.1
}

pub fn data() -> &'static ObjectStoreStatic {
    &BUNDLED_DATA.0
}