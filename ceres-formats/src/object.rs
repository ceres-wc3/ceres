use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::metadata::FieldVariant;
use crate::metadata::MetadataStore;
use crate::ObjectId;
use crate::ObjectKind;
use crate::parser::slk;
use crate::ValueType;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    String(String),
    Int(i32),
    Real(f32),
    Unreal(f32),
}

impl Value {
    pub fn from_str_and_ty(value: &str, ty: ValueType) -> Option<Self> {
        Some(match ty {
            ValueType::Unreal => Value::Unreal(value.parse().ok()?),
            ValueType::Real => Value::Real(value.parse().ok()?),
            ValueType::Int => Value::Int(value.parse().ok()?),
            ValueType::String => Value::String(value.into()),
        })
    }

    pub fn type_id(&self) -> u32 {
        match self {
            Value::Int(..) => 0,
            Value::Real(..) => 1,
            Value::Unreal(..) => 2,
            Value::String(..) => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeveledValue {
    pub level: u32,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldKind {
    Simple { value: Value },
    Leveled { values: Vec<LeveledValue> },
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
    dirty: bool,
}

impl Object {
    pub fn new(id: ObjectId, kind: ObjectKind) -> Object {
        Object {
            id,
            kind,
            parent_id: None,
            fields: Default::default(),
            dirty: true,
        }
    }

    pub fn with_parent(id: ObjectId, parent_id: ObjectId, kind: ObjectKind) -> Object {
        Object {
            id,
            kind,
            parent_id: Some(parent_id),
            fields: Default::default(),
            dirty: true,
        }
    }

    pub fn id(&self) -> ObjectId {
        self.id
    }

    pub fn set_id(&mut self, id: ObjectId) {
        self.id = id
    }

    pub fn parent_id(&self) -> Option<ObjectId> {
        self.parent_id
    }

    pub fn set_parent_id(&mut self, parent_id: Option<ObjectId>) {
        self.parent_id = parent_id
    }

    pub fn kind(&self) -> ObjectKind {
        self.kind
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn set_dirty(&mut self, dirty: bool) {
        self.dirty = dirty
    }

    pub fn fields(&self) -> impl Iterator<Item = (&ObjectId, &Field)> {
        self.fields.iter()
    }

    pub fn field(&self, id: ObjectId) -> Option<&Field> {
        self.fields.get(&id)
    }

    pub fn simple_field(&self, id: ObjectId) -> Option<&Value> {
        self.fields.get(&id).and_then(|field| match &field.kind {
            FieldKind::Simple { value } => Some(value),
            _ => None,
        })
    }

    pub fn leveled_field(&self, id: ObjectId, level: u32) -> Option<&Value> {
        self.fields.get(&id).and_then(|field| match &field.kind {
            FieldKind::Leveled { values } => values
                .iter()
                .find(|value| value.level == level)
                .map(|value| &value.value),
            _ => None,
        })
    }

    pub fn unset_simple_field(&mut self, id: ObjectId) {
        self.dirty = true;

        self.fields.remove(&id);
    }

    pub fn unset_leveled_field(&mut self, id: ObjectId, level: u32) {
        self.dirty = true;

        if let Some(field) = self.fields.get_mut(&id) {
            if let FieldKind::Leveled { values } = &mut field.kind {
                values.retain(|dv| dv.level != level)
            }
        }
    }

    pub fn set_simple_field(&mut self, id: ObjectId, value: Value) {
        self.dirty = true;

        let kind = FieldKind::Simple { value };
        let field = Field { id, kind };
        self.fields.insert(id, field);
    }

    pub fn set_leveled_field(&mut self, id: ObjectId, level: u32, value: Value) {
        self.dirty = true;

        let field = self.fields.entry(id).or_insert_with(|| Field {
            id,
            kind: FieldKind::Leveled {
                values: Default::default(),
            },
        });

        match &mut field.kind {
            FieldKind::Simple { .. } => eprintln!(
                "tried to insert data field {} for object {}, but a simple field {} already exists",
                id, self.id, field.id
            ),
            FieldKind::Leveled { values } => {
                let new_value = LeveledValue { level, value };

                if let Some(value) = values.iter_mut().find(|dv| dv.level == level) {
                    *value = new_value;
                } else {
                    values.push(new_value);
                }
            }
        }
    }

    /// Merges this object's data with another object's data
    /// Doesn't do field-level merging because it's not needed
    /// in our use case. Just override the fields in this object
    /// from the fields in the other.
    pub fn add_from(&mut self, other: &Object) {
        self.dirty = true;

        for (id, field) in &other.fields {
            self.fields.insert(*id, field.clone());
        }
    }

    pub(crate) fn process_slk_field(
        &mut self,
        value: &slk::Value,
        name: &str,
        metadata: &MetadataStore,
    ) -> Option<()> {
        let (field_meta, level) = metadata.query_slk_field(name, &self)?;

        let value = Value::from_str_and_ty(value.as_inner()?, field_meta.value_ty)?;
        let field_id = field_meta.id;

        match field_meta.variant {
            FieldVariant::Normal { .. } => self.set_simple_field(field_id, value),
            FieldVariant::Leveled { .. } | FieldVariant::Data { .. } => self.set_leveled_field(
                field_id,
                level.expect("field must have level specified"),
                value,
            ),
        }

        Some(())
    }

    pub(crate) fn process_func_field(
        &mut self,
        key: &str,
        value: &str,
        index: i8,
        metadata: &MetadataStore,
    ) -> Option<()> {
        let (field_meta, level) = metadata.query_profile_field(key, &self, index)?;
        let value = Value::from_str_and_ty(value, field_meta.value_ty)?;

        if let Some(level) = level {
            self.set_leveled_field(field_meta.id, level, value)
        } else {
            self.set_simple_field(field_meta.id, value)
        }

        Some(())
    }
}
