use std::collections::BTreeMap;
use std::iter::Extend;
use std::path::Path;
use std::fs;

use serde::{Serialize, Deserialize};

use crate::ObjectId;
use crate::ObjectKind;
use crate::metadata::MetadataStore;
use crate::metadata::FieldVariant;
use crate::parser::profile;
use crate::parser::slk;
use crate::parser::slk::read_row_num;
use crate::parser::slk::read_row_str;

#[derive(Debug, Serialize, Deserialize)]
pub enum Value {
    Str(String),
    Int(i32),
    Real(f32),
    Unreal(f32),
    Empty,
}

impl Value {
    pub fn from_str_and_ty(ty: &str, value: &str) -> Option<Self> {
        Some(match ty {
            "unreal" => Value::Unreal(value.parse().ok()?),
            "real" => Value::Real(value.parse().ok()?),
            "int" | "bool" => Value::Int(value.parse().ok()?),
            _ => Value::Str(value.into()),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Field {
    Simple {
        value: Value,
    },
    Data {
        data_id: u8,
        level:   u32,
        value:   Value,
    },
}

impl Field {
    pub fn simple(value: Value) -> Field {
        Field::Simple { value }
    }

    pub fn data(value: Value, data_id: u8, level: u32) -> Field {
        Field::Data {
            value,
            level,
            data_id,
        }
    }

    pub fn value(&self) -> &Value {
        match self {
            Field::Simple { value } => value,
            Field::Data { value, .. } => value,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Object {
    kind:      ObjectKind,
    id:        ObjectId,
    parent_id: Option<ObjectId>,
    fields:    BTreeMap<ObjectId, Field>,
}

impl Object {
    pub fn new(id: ObjectId, kind: ObjectKind) -> Object {
        Object {
            id,
            kind,
            parent_id: None,
            fields: Default::default(),
        }
    }

    pub fn with_parent(id: ObjectId, parent_id: ObjectId, kind: ObjectKind) -> Object {
        Object {
            id,
            kind,
            parent_id: Some(parent_id),
            fields: Default::default(),
        }
    }

    pub fn id(&self) -> ObjectId {
        self.id
    }

    pub fn kind(&self) -> ObjectKind {
        self.kind
    }

    pub fn fields(&self) -> impl Iterator<Item = (&ObjectId, &Field)> {
        self.fields.iter()
    }

    pub fn field(&self, id: ObjectId) -> Option<&Field> {
        self.fields.get(&id)
    }

    pub fn set_field(&mut self, id: ObjectId, field: Field) {
        self.fields.insert(id, field);
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectStore {
    objects: BTreeMap<ObjectId, Object>,
}

impl Default for ObjectStore {
    fn default() -> ObjectStore {
        ObjectStore {
            objects: Default::default(),
        }
    }
}

fn process_slk_field(
    object: &mut Object,
    value: &slk::Value,
    name: &str,
    metadata: &MetadataStore,
) -> Option<()> {
    let (field_meta, level) = metadata.query_slk_field(name, &object)?;
    let value = Value::from_str_and_ty(&field_meta.value_ty, value.as_inner()?)?;

    let field = match field_meta.variant {
        FieldVariant::Normal { .. } => Field::simple(value),
        FieldVariant::Leveled { .. } => Field::data(value, 0, level.unwrap()),
        FieldVariant::Data { id } => Field::data(value, id, level.unwrap()),
    };

    object.set_field(field_meta.id, field);

    Some(())
}

fn process_func_field(
    object: &mut Object,
    key: &str,
    value: &str,
    index: i8,
    metadata: &MetadataStore,
) -> Option<()> {
    let field_meta = metadata.query_profile_field(key, &object, index)?;
    let value = Value::from_str_and_ty(&field_meta.value_ty, value)?;

    let field = match field_meta.variant {
        FieldVariant::Normal { .. } => Field::simple(value),
        FieldVariant::Leveled { .. } => Field::data(value, 0, 0),
        FieldVariant::Data { id } => Field::data(value, id, 0),
    };

    object.set_field(field_meta.id, field);

    Some(())
}

impl ObjectStore {
    pub fn objects(&self) -> impl Iterator<Item = &Object> {
        self.objects.values()
    }

    pub fn object(&self, id: ObjectId) -> Option<&Object> {
        self.objects.get(&id)
    }

    pub fn insert_object(&mut self, object: Object) {
        self.objects.insert(object.id, object);
    }

    pub fn merge_with(self, mut other: ObjectStore) -> ObjectStore {
        other.objects.extend(self.objects.into_iter());

        other
    }

    pub fn insert_slk_row<'src>(
        &mut self,
        kind: ObjectKind,
        row: slk::Row<'src>,
        legend: &slk::Legend<'src>,
        metadata: &MetadataStore,
    ) -> Option<()> {
        let id = row
            .cells
            .get(0)
            .and_then(|c| c.value().as_str())
            .and_then(|id| ObjectId::from_bytes(id.as_bytes()))?;

        let object = if kind == ObjectKind::empty() {
            self.objects.get_mut(&id)?
        } else {
            self.objects
                .entry(id)
                .or_insert_with(|| Object::new(id, kind))
        };

        for (value, name) in row
            .cells
            .iter()
            .filter_map(|cell| legend.name_by_cell(&cell).map(|name| (cell.value(), name)))
        {
            process_slk_field(object, value, name, metadata);
        }

        Some(())
    }

    fn insert_func_entry(&mut self, entry: profile::Entry, metadata: &MetadataStore) -> Option<()> {
        let id = ObjectId::from_bytes(entry.id.as_bytes())?;
        let object = self.objects.get_mut(&id)?;

        for (key, values) in entry.values {
            for (index, value) in values.split(',').enumerate() {
                process_func_field(object, key, value, index as i8, metadata);
            }
        }

        Some(())
    }
}

fn read_func_file<P: AsRef<Path>>(path: P, metadata: &MetadataStore, data: &mut ObjectStore) {
    dbg!(path.as_ref());

    let src = fs::read(path).unwrap();
    let entries = profile::Entries::new(&src);

    for entry in entries {
        data.insert_func_entry(entry, metadata);
    }
}

pub fn read_slk_file<P: AsRef<Path>>(
    path: P,
    kind: ObjectKind,
    metadata: &MetadataStore,
    data: &mut ObjectStore,
) {
    dbg!(path.as_ref());
    let src = fs::read(path).unwrap();
    let mut table = slk::Table::new(&src).unwrap();
    let legend = table.legend();

    while table.has_next() {
        if let Some(row) = table.next_row() {
            data.insert_slk_row(kind, row, &legend, metadata);
        }
    }
}

pub fn read_data_dir<P: AsRef<Path>>(path: P, metadata: &MetadataStore) -> ObjectStore {
    let path = path.as_ref();
    let mut data = ObjectStore::default();

    const SLKS: &[(ObjectKind, &str)] = &[
        (ObjectKind::UNIT, "units/unitdata.slk"),
        (ObjectKind::ABILITY, "units/abilitydata.slk"),
        (ObjectKind::ITEM, "units/itemdata.slk"),
        (ObjectKind::BUFF, "units/abilitybuffdata.slk"),
        (ObjectKind::DESTRUCTABLE, "units/destructabledata.slk"),
        (ObjectKind::UPGRADE, "units/upgradedata.slk"),
        (ObjectKind::empty(), "units/unitbalance.slk"),
        (ObjectKind::empty(), "units/unitabilities.slk"),
        (ObjectKind::empty(), "units/unitweapons.slk"),
        (ObjectKind::empty(), "units/unitui.slk"),
    ];

    for (kind, file_path) in SLKS {
        read_slk_file(path.join(file_path), *kind, &metadata, &mut data);
    }

    const PROFILES: &[&str] = &[
        "units/campaignabilityfunc.txt",
        "units/campaignunitfunc.txt",
        "units/campaignupgradefunc.txt",
        "units/commandfunc.txt",
        "units/commonabilityfunc.txt",
        "units/humanabilityfunc.txt",
        "units/humanunitfunc.txt",
        "units/humanupgradefunc.txt",
        "units/itemabilityfunc.txt",
        "units/itemfunc.txt",
        "units/miscdata.txt",
        "units/miscgame.txt",
        "units/neutralabilityfunc.txt",
        "units/neutralunitfunc.txt",
        "units/neutralupgradefunc.txt",
        "units/nightelfabilityfunc.txt",
        "units/nightelfunitfunc.txt",
        "units/nightelfupgradefunc.txt",
        "units/orcabilityfunc.txt",
        "units/orcunitfunc.txt",
        "units/orcupgradefunc.txt",
        "units/undeadabilityfunc.txt",
        "units/undeadunitfunc.txt",
        "units/undeadupgradefunc.txt",
    ];

    for file_path in PROFILES {
        read_func_file(path.join(file_path), &metadata, &mut data);
    }

    data
}
