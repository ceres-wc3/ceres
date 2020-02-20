use ceres_formats::metadata::MetadataStore;
use ceres_formats::objectstore::ObjectStoreStock;
use lazy_static::lazy_static;

const BUNDLED_DATA_BIN: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/data.bin"));

lazy_static! {
    static ref BUNDLED_DATA: (ObjectStoreStock, MetadataStore) = {
        let data: (ObjectStoreStock, MetadataStore) =
            bincode::deserialize(BUNDLED_DATA_BIN).unwrap();

        data
    };
}

pub fn metadata() -> &'static MetadataStore {
    &BUNDLED_DATA.1
}

pub fn data() -> &'static ObjectStoreStock {
    &BUNDLED_DATA.0
}
