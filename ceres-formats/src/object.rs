use std::collections::BTreeMap;
use std::fs;
use std::iter::Extend;
use std::path::Path;
use std::rc::Rc;
use std::cell::{RefCell, Ref};
use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::metadata::FieldVariant;
use crate::metadata::MetadataStore;
use crate::ObjectId;
use crate::ObjectKind;
use crate::parser::profile;
use crate::parser::slk;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Str(String),
    Int(i32),
    Real(f32),
    Unreal(f32),
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

    pub fn type_id(&self) -> u32 {
        match self {
            Value::Int(..) => 0,
            Value::Real(..) => 1,
            Value::Unreal(..) => 2,
            Value::Str(..) => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataValue {
    pub data_id: u8,
    pub level:   u32,
    pub value:   Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldKind {
    Simple { value: Value },
    Data { values: Vec<DataValue> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub id:   ObjectId,
    pub kind: FieldKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    pub fn parent_id(&self) -> Option<ObjectId> {
        self.parent_id
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

    pub fn set_simple_field(&mut self, id: ObjectId, value: Value) {
        let kind = FieldKind::Simple { value };
        let field = Field { id, kind };
        self.fields.insert(id, field);
    }

    pub fn set_level_field(&mut self, id: ObjectId, level: u32, value: Value) {
        let field = self.fields.entry(id).or_insert_with(|| Field {
            id,
            kind: FieldKind::Data {
                values: Default::default(),
            },
        });

        match &mut field.kind {
            FieldKind::Simple { .. } => eprintln!(
                "tried to insert data field {} for object {}, but a simple field {} already exists",
                id, self.id, field.id
            ),
            FieldKind::Data { values } => {
                let new_value = DataValue {
                    data_id: 0,
                    level,
                    value,
                };

                if let Some(value) = values
                    .iter_mut()
                    .find(|dv| dv.level == level && dv.data_id == 0)
                {
                    *value = new_value;
                } else {
                    values.push(new_value);
                }
            }
        }
    }

    pub fn set_data_field(&mut self, id: ObjectId, level: u32, data: u8, value: Value) {
        let field = self.fields.entry(id).or_insert_with(|| Field {
            id,
            kind: FieldKind::Data {
                values: Default::default(),
            },
        });

        match &mut field.kind {
            FieldKind::Simple { .. } => eprintln!(
                "tried to insert data field {} for object {}, but a simple field {} already exists",
                id, self.id, field.id
            ),
            FieldKind::Data { values } => {
                let new_value = DataValue {
                    data_id: data,
                    level,
                    value,
                };

                if let Some(value) = values
                    .iter_mut()
                    .find(|dv| dv.level == level && dv.data_id == data)
                {
                    *value = new_value;
                } else {
                    values.push(new_value);
                }
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectStore {
    objects: BTreeMap<ObjectId, Rc<RefCell<Object>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectStoreStatic {
    objects: BTreeMap<ObjectId, Object>
}

impl Default for ObjectStore {
    fn default() -> ObjectStore {
        ObjectStore {
            objects: Default::default(),
        }
    }
}

impl Default for ObjectStoreStatic {
    fn default() -> ObjectStoreStatic {
        ObjectStoreStatic {
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
    let field_id = field_meta.id;

    match field_meta.variant {
        FieldVariant::Normal { .. } => object.set_simple_field(field_id, value),
        FieldVariant::Leveled { .. } => object.set_level_field(
            field_id,
            level.expect("field must have level specified"),
            value,
        ),
        FieldVariant::Data { id } => object.set_data_field(
            field_id,
            level.expect("field must have level specified"),
            id,
            value,
        ),
    }

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
    let field_id = field_meta.id;

    match field_meta.variant {
        FieldVariant::Normal { .. } => object.set_simple_field(field_id, value),
        FieldVariant::Leveled { .. } => object.set_level_field(field_id, 0, value),
        FieldVariant::Data { id } => object.set_data_field(field_id, 0, id, value),
    }

    Some(())
}

impl ObjectStore {
    pub fn objects(&self) -> impl Iterator<Item = &Rc<RefCell<Object>>> {
        self.objects.values()
    }

    pub fn object(&self, id: ObjectId) -> Option<&Rc<RefCell<Object>>> {
        self.objects.get(&id)
    }

    pub fn insert_object(&mut self, object: Object) {
        self.objects.insert(object.id, Rc::new(RefCell::new(object)));
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
                .or_insert_with(|| Rc::new(RefCell::new(Object::new(id, kind))))
        };

        for (value, name) in row
            .cells
            .iter()
            .filter_map(|cell| legend.name_by_cell(&cell).map(|name| (cell.value(), name)))
        {
            process_slk_field(&mut object.borrow_mut(), value, name, metadata);
        }

        Some(())
    }

    fn insert_func_entry(&mut self, entry: profile::Entry, metadata: &MetadataStore) -> Option<()> {
        let id = ObjectId::from_bytes(entry.id.as_bytes())?;
        let object = self.objects.get_mut(&id)?;

        for (key, values) in entry.values {
            for (index, value) in values.split(',').enumerate() {
                process_func_field(&mut object.borrow_mut(), key, value, index as i8, metadata);
            }
        }

        Some(())
    }
}

impl ObjectStoreStatic {
    pub fn new(data: &ObjectStore) -> ObjectStoreStatic {
        let mut data_static = Self::default();
        data_static.merge_from(data);
        data_static
    }

    fn merge_from(&mut self, data: &ObjectStore) {
        for object in data.objects() {
            let object = object.borrow().clone();

            self.objects.insert(object.id, object);
        }
    }

    pub fn object(&self, id: ObjectId) -> Option<&Object> {
        self.objects.get(&id)
    }

    pub fn objects(&self) -> impl Iterator<Item=&Object> {
        self.objects.values()
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

fn read_slk_file<P: AsRef<Path>>(
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
        (ObjectKind::DOODAD, "doodads/doodads.slk"),
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
