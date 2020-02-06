use std::fs;
use std::path::Path;
use std::collections::BTreeMap;
use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use slotmap::{new_key_type, DenseSlotMap};

use crate::parser::slk;
use crate::parser::slk::read_row_num;
use crate::parser::slk::read_row_str;
use crate::ObjectId;
use crate::ObjectKind;
use crate::ValueType;
use crate::object::Object;

new_key_type! {
    struct FieldKey;
}

struct BasicInfo {
    field_id:   ObjectId,
    field_name: String,
    value_ty:   String,
    index:      i8,
    is_profile: bool,
}

fn read_basic_info<'src>(row: &slk::Row<'src>, legend: &slk::Legend<'src>) -> BasicInfo {
    let field_id = read_row_str(&row, legend, "ID").unwrap();
    let field_name: String = read_row_str(&row, legend, "field").unwrap().into();
    let value_ty: String = read_row_str(&row, legend, "type").unwrap().into();
    let index: i8 = read_row_num(&row, legend, "index").unwrap_or(-1);
    let slk = read_row_str(&row, legend, "slk").unwrap();

    let field_id = ObjectId::from_bytes(field_id.as_bytes()).unwrap();

    BasicInfo {
        field_id,
        field_name,
        value_ty,
        index,
        is_profile: slk == "Profile",
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldDesc {
    pub id:           ObjectId,
    pub index:        i8,
    pub variant:      FieldVariant,
    pub value_ty:     ValueType,
    pub value_ty_raw: String,
    pub exclusive:    Option<Vec<ObjectId>>,
    pub kind:         ObjectKind,
    pub is_profile:   bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FieldVariant {
    Normal { name: String },
    Leveled { name: String },
    Data { id: u8 },
}

impl FieldVariant {
    pub fn name(&self) -> &str {
        match self {
            FieldVariant::Normal { name } => name,
            FieldVariant::Leveled { name } => name,
            FieldVariant::Data { .. } => "data",
        }
    }

    pub fn is_normal(&self) -> bool {
        match self {
            FieldVariant::Normal { .. } => true,
            _ => false,
        }
    }

    pub fn is_leveled(&self) -> bool {
        match self {
            FieldVariant::Leveled { .. } => true,
            FieldVariant::Data { .. } => true,
            _ => false,
        }
    }

    pub fn is_data(&self) -> bool {
        match self {
            FieldVariant::Data { .. } => true,
            _ => false,
        }
    }

    pub fn data_id(&self) -> Option<u8> {
        match self {
            FieldVariant::Data { id } => Some(*id),
            _ => None,
        }
    }
}

fn split_by_digits(input: &str) -> Option<(&str, &str)> {
    input
        .find(|c: char| c.is_digit(10))
        .map(|i| (&input[0..i], &input[i..input.len()]))
}

fn data_char_to_id(input: u8) -> u8 {
    match input {
        b'a' | b'A' => 1,
        b'b' | b'B' => 2,
        b'c' | b'C' => 3,
        b'd' | b'D' => 4,
        b'e' | b'E' => 5,
        b'f' | b'F' => 6,
        b'g' | b'G' => 7,
        b'h' | b'H' => 8,
        b'i' | b'I' => 9,
        b'j' | b'J' => 10,
        _ => panic!("unknown data field id"),
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct MetadataStore {
    // primary store for the fields
    // other collections in this struct hold references to FieldKeys returned by this
    fields:            DenseSlotMap<FieldKey, FieldDesc>,
    // for fields that are only present on certain objects (namely, ability Data fields),
    // this holds the association between objects and fields that are available only on them
    objects_with_data: HashMap<ObjectId, Vec<ObjectId>>,
    // mapping between field ids and field keys
    ids_to_keys:       HashMap<ObjectId, FieldKey>,
    // mapping between field names and field keys
    // multiple fields may be mapped to the same name,
    // namely if they belong to different objects or have different indices
    // so additional filtering must be performed
    names_to_keys:     BTreeMap<String, Vec<FieldKey>>,
}

impl MetadataStore {
    fn add_field(&mut self, field: FieldDesc) {
        let id = field.id;
        let name = field.variant.name().to_ascii_lowercase();
        let key = self.fields.insert(field);
        self.ids_to_keys.insert(id, key);
        self.names_to_keys.entry(name).or_default().push(key);
    }

    fn insert_basic_field<'src>(
        &mut self,
        row: slk::Row<'src>,
        legend: &slk::Legend<'src>,
        kind: ObjectKind,
    ) {
        let basic_info = read_basic_info(&row, &legend);

        let repeat = read_row_num::<u8>(&row, legend, "repeat");
        let mut leveled = false;
        if let Some(repeat) = repeat {
            leveled = repeat != 0;
        }

        let variant = if leveled {
            FieldVariant::Leveled {
                name: basic_info.field_name.clone(),
            }
        } else {
            FieldVariant::Normal {
                name: basic_info.field_name.clone(),
            }
        };

        self.add_field(FieldDesc {
            id: basic_info.field_id,
            index: basic_info.index,
            value_ty: ValueType::new(&basic_info.value_ty),
            value_ty_raw: basic_info.value_ty,
            exclusive: None,
            variant,
            kind,
            is_profile: basic_info.is_profile,
        });
    }

    fn insert_unit_row<'src>(&mut self, row: slk::Row<'src>, legend: &slk::Legend<'src>) {
        let basic_info = read_basic_info(&row, &legend);

        let use_unit: u8 = read_row_num(&row, legend, "useUnit").unwrap_or(0);
        let use_bld: u8 = read_row_num(&row, legend, "useBuilding").unwrap_or(0);
        let use_hero: u8 = read_row_num(&row, legend, "useHero").unwrap_or(0);
        let use_item: u8 = read_row_num(&row, legend, "useItem").unwrap_or(0);

        let mut kind = ObjectKind::empty();
        if use_item != 0 {
            kind |= ObjectKind::ITEM
        }

        if (use_unit != 0) || (use_bld != 0) || (use_hero != 0) {
            kind |= ObjectKind::UNIT
        }

        let variant = FieldVariant::Normal {
            name: basic_info.field_name.clone(),
        };

        self.add_field(FieldDesc {
            id: basic_info.field_id,
            index: basic_info.index,
            value_ty: ValueType::new(&basic_info.value_ty),
            value_ty_raw: basic_info.value_ty,
            exclusive: None,
            variant,
            kind,
            is_profile: basic_info.is_profile,
        });
    }

    fn insert_ability_row<'src>(&mut self, row: slk::Row<'src>, legend: &slk::Legend<'src>) {
        let basic_info = read_basic_info(&row, &legend);

        let repeat = read_row_num::<u8>(&row, legend, "repeat");
        let data_id = read_row_num::<u8>(&row, legend, "data");
        let exclusive = read_row_str(&row, legend, "useSpecific");

        let mut leveled = false;
        if let Some(repeat) = repeat {
            leveled = repeat != 0;
        }

        let variant = if basic_info.field_name == "Data" {
            if data_id.is_none() {
                return;
            }

            let data_id = data_id.unwrap();

            FieldVariant::Data { id: data_id }
        } else if leveled {
            FieldVariant::Leveled {
                name: basic_info.field_name.clone(),
            }
        } else {
            FieldVariant::Normal {
                name: basic_info.field_name.clone(),
            }
        };

        let exclusive = exclusive.map(|e| {
            let list = e
                .split(',')
                .filter_map(|s| ObjectId::from_bytes(s.as_bytes()))
                .collect::<Vec<_>>();

            for object_id in &list {
                self.objects_with_data
                    .entry(*object_id)
                    .or_default()
                    .push(basic_info.field_id);
            }

            list
        });

        self.add_field(FieldDesc {
            id: basic_info.field_id,
            value_ty: ValueType::new(&basic_info.value_ty),
            value_ty_raw: basic_info.value_ty,
            variant,
            exclusive,
            index: basic_info.index,
            kind: ObjectKind::ABILITY,
            is_profile: basic_info.is_profile,
        });
    }

    fn find_data_field(&self, object_id: ObjectId, data_id: u8) -> Option<&FieldDesc> {
        self.objects_with_data.get(&object_id).and_then(|v| {
            self.find_field(
                v.iter().filter_map(|id| self.ids_to_keys.get(id)).copied(),
                |f| match f.variant {
                    FieldVariant::Data { id } => id == data_id,
                    _ => false,
                },
            )
        })
    }

    fn find_named_field<F>(&self, name: &str, closure: F) -> Option<&FieldDesc>
    where
        F: FnMut(&FieldDesc) -> bool,
    {
        self.names_to_keys
            .get(&name.to_ascii_lowercase())
            .and_then(|v| self.find_field(v.iter().copied(), closure))
    }

    fn find_field<I, F>(&self, iter: I, mut closure: F) -> Option<&FieldDesc>
    where
        I: Iterator<Item = FieldKey>,
        F: FnMut(&FieldDesc) -> bool,
    {
        iter.filter_map(|k| self.fields.get(k)).find(|f| closure(f))
    }

    /// Queries an SLK field by it's name and target object.
    /// The object is necessary because the same field name can
    /// resolve to different fields depending on the object.
    pub fn query_slk_field(
        &self,
        field_name: &str,
        object: &Object,
    ) -> Option<(&FieldDesc, Option<u32>)> {
        let object_kind = object.kind();
        let object_id = object.id();
        self.find_named_field(&field_name, |f| {
            !f.is_profile && f.kind.contains(object_kind)
        })
        .map(|f| (f, None))
        .or_else(|| {
            split_by_digits(&field_name).and_then(|(name, raw_level)| {
                let level: u32 = raw_level.parse().unwrap();

                let field = if name.starts_with("Data") {
                    let data_id = data_char_to_id(name.as_bytes()[4]);
                    self.find_data_field(object_id, data_id)
                } else {
                    self.find_named_field(name, |f| !f.is_profile && f.kind.contains(object_kind))
                };

                field.map(|f| (f, Some(level)))
            })
        })
    }

    /// Queries a Profile field by it's name and target object.
    /// The object is necessary because the same field name can
    /// resolve to different fields depending on the object.
    pub fn query_profile_field(
        &self,
        field_name: &str,
        object: &Object,
        index: i8,
    ) -> Option<(&FieldDesc, Option<u32>)> {
        let object_kind = object.kind();

        self.find_named_field(&field_name, |f| {
            f.is_profile
                && f.kind.contains(object_kind)
                && ((f.variant.is_normal() && (f.index == index || f.index == -1))
                    || (f.variant.is_leveled() && (f.index == 0)))
        })
        .map(|f| {
            if f.variant.is_leveled() {
                (f, Some(index as u32))
            } else {
                (f, None)
            }
        })
        .or_else(|| None)
    }

    pub fn query_object_field(&self, field_id: ObjectId, object: &Object) -> Option<&FieldDesc> {
        let desc = self
            .ids_to_keys
            .get(&field_id)
            .and_then(|k| self.fields.get(*k))?;

        if !desc.kind.contains(object.kind()) {
            return None;
        }

        if let Some(exclusive) = &desc.exclusive {
            if !exclusive.contains(&object.id()) {
                return None;
            }
        }

        Some(desc)
    }

    /// Will return an iterator of all fields available for this object,
    /// irrespective of which fields exist on the object itself.
    pub fn query_all_object_fields(&self, object: &Object) -> impl Iterator<Item = &FieldDesc> {
        let object_kind = object.kind();
        let object_id = if let Some(id) = object.parent_id() {
            id
        } else {
            object.id()
        };

        self.fields
            .values()
            .filter(move |desc| desc.kind.contains(object_kind))
            .filter(move |desc| {
                if let Some(exclusive) = &desc.exclusive {
                    exclusive.contains(&object_id)
                } else {
                    true
                }
            })
    }

    /// First tries to fetch a Profile/Func field in the format of
    /// <field_name><index id>, e.g. Buttonpos1 resolving to abpx for units, and Buttonpos2 resolving to abpy,
    /// if that fails then will try to grab an slk field using the regular approach
    pub fn query_lua_field(
        &self,
        object: &Object,
        name: &str,
    ) -> Option<(&FieldDesc, Option<u32>)> {
        split_by_digits(name)
            .map(|(name, index)| {
                (
                    name,
                    index
                        .parse()
                        .expect("field must not have leading non-numeric characters after digits"),
                )
            })
            .or_else(|| Some((name, 0)))
            .and_then(|(name, index)| self.query_profile_field(name, object, index))
            .or_else(|| self.query_slk_field(name, object))
    }

    pub fn field_by_id(&self, id: ObjectId) -> Option<&FieldDesc> {
        let field_key = self.ids_to_keys.get(&id)?;
        self.fields.get(*field_key)
    }
}

pub fn read_metadata_dir<P: AsRef<Path>>(path: P) -> MetadataStore {
    let path = path.as_ref();
    let mut metadata = MetadataStore::default();

    read_metadata_file(path.join("units/unitmetadata.slk"), |row, legend| {
        metadata.insert_unit_row(row, legend);
    });

    read_metadata_file(path.join("units/abilitymetadata.slk"), |row, legend| {
        metadata.insert_ability_row(row, legend);
    });

    read_metadata_file(path.join("units/abilitybuffmetadata.slk"), |row, legend| {
        metadata.insert_basic_field(row, legend, ObjectKind::BUFF);
    });

    read_metadata_file(path.join("units/upgrademetadata.slk"), |row, legend| {
        metadata.insert_basic_field(row, legend, ObjectKind::UPGRADE);
    });

    read_metadata_file(
        path.join("units/destructablemetadata.slk"),
        |row, legend| {
            metadata.insert_basic_field(row, legend, ObjectKind::DESTRUCTABLE);
        },
    );

    read_metadata_file(path.join("units/miscmetadata.slk"), |row, legend| {
        metadata.insert_basic_field(row, legend, ObjectKind::MISC);
    });

    read_metadata_file(path.join("doodads/doodadmetadata.slk"), |row, legend| {
        metadata.insert_basic_field(row, legend, ObjectKind::DOODAD);
    });

    // string id field for unit names
    metadata.add_field(FieldDesc {
        id:           ObjectId::from_bytes(b"siid").unwrap(),
        index:        0,
        variant:      FieldVariant::Normal {
            name: "name".to_string(),
        },
        value_ty:     ValueType::String,
        value_ty_raw: "string".to_string(),
        exclusive:    None,
        kind:         ObjectKind::UNIT,
        is_profile:   false,
    });

    metadata
}

fn read_metadata_file<C, P>(path: P, mut callback: C)
where
    C: FnMut(slk::Row, &slk::Legend) -> (),
    P: AsRef<Path>,
{
    let src = fs::read(path).unwrap();
    let table = slk::Table::new(&src);

    if table.is_none() {
        return;
    }

    let mut table = table.unwrap();
    let legend = table.legend();

    while table.has_next() {
        if let Some(row) = table.next_row() {
            callback(row, &legend)
        }
    }
}
