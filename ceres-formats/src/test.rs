use std::env;
use crate::*;
use crate::metadata::MetadataStore;
use crate::object::ObjectStore;

fn print_objects(data: &ObjectStore, metadata: &MetadataStore) {
    for (i, object) in data.objects().enumerate() {
        println!("{} {:?} {:?}:", i, object.kind(), object.id());
        for (id, field) in object.fields() {
            let field_meta = metadata.field_by_id(*id).unwrap();
            println!(
                "    {} {} = {:?}",
                id,
                field_meta.variant.name(),
                field.value()
            );
        }
    }
}

pub fn main() {
    let args: Vec<String> = env::args().collect();

    let filename = &args[1];
    let metadata = crate::metadata::read_metadata_dir(filename);

    let pretty_config = ron::ser::PrettyConfig {
        depth_limit:            10,
        new_line:               "\n".into(),
        indentor:               "    ".into(),
        separate_tuple_members: false,
        enumerate_arrays:       false,
    };

    let data = crate::object::read_data_dir(filename, &metadata);

    // println!("{}", ron::ser::to_string_pretty(&(&data, &metadata), pretty_config).unwrap());
    // print_objects(&data, &metadata);

    // let object_id = ObjectId::from_bytes(b"AHbz").unwrap();
    // let object = object::Object::new(object_id, ObjectKind::ABILITY);

    // println!("{:#?}", metadata.query_slk_field("DataB9", &object));
    // println!("{:#?}", metadata.query_slk_field("Cast10", &object));
    // println!("{:#?}", metadata.query_slk_field("race", &object));

    // serde_json::to_writer_pretty(std::io::stdout().lock(), &metadata).unwrap();
    bincode::serialize_into(std::io::stdout().lock(), &(&data, &metadata)).unwrap();
}
