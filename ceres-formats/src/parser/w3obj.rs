use crate::ObjectKind;

fn is_type_with_data(kind: ObjectKind) -> bool {
    match kind {
        ObjectKind::DOODAD | ObjectKind::ABILITY | ObjectKind::UPGRADE => true,
        _ => false,
    }
}

pub mod read {
    use byteorder::{BE, LE, ReadBytesExt};

    use crate::{ObjectId, ObjectKind};
    use crate::error::*;
    use crate::object::{Object, ObjectStore, Value};

    use super::is_type_with_data;

    fn read_str<'src>(source: &mut &'src [u8]) -> Result<&'src [u8], ObjParseError> {
        let end = source
            .iter()
            .enumerate()
            .find(|(_, c)| **c == 0)
            .map(|(i, _)| i)
            .ok_or_else(ObjParseError::unterminated_string)?;
        let result = &source[..end];
        *source = &source[end + 1..];

        Ok(result)
    }

    fn read_value(source: &mut &[u8], field_type: u32) -> Result<Value, ObjParseError> {
        Ok(match field_type {
            0 => Value::Int(source.read_i32::<LE>()?),
            1 => Value::Real(source.read_f32::<LE>()?),
            2 => Value::Unreal(source.read_f32::<LE>()?),
            3 => Value::String(read_str(source).map(|s| String::from_utf8_lossy(s).into_owned())?),
            _ => panic!("malformed"),
        })
    }

    fn read_field(
        source: &mut &[u8],
        object: &mut Object,
        uses_extra_ints: bool,
    ) -> Result<(), ObjParseError> {
        let field_id = source.read_u32::<BE>().map(ObjectId::new)?;
        let field_type = source.read_u32::<LE>()?;

        if !uses_extra_ints {
            let value = read_value(source, field_type)?;

            object.set_simple_field(field_id, value);
        } else {
            let level = source.read_u32::<LE>()?;
            let data_id = source.read_u32::<LE>()?;
            let value = read_value(source, field_type)?;

            object.set_data_field(field_id, level, data_id as u8, value);
        }

        // read trailing int
        source.read_u32::<LE>()?;

        Ok(())
    }

    pub fn read_object_table(
        source: &mut &[u8],
        kind: ObjectKind,
    ) -> Result<ObjectStore, ObjParseError> {
        let obj_amount = source.read_u32::<LE>()?;
        let mut objects = ObjectStore::default();

        for _ in 0..obj_amount {
            let original_id = source.read_u32::<BE>().map(ObjectId::new)?;
            let new_id = source.read_u32::<BE>().map(ObjectId::new)?;

            let mut object = if new_id.to_u32() != 0 {
                Object::with_parent(new_id, original_id, kind)
            } else {
                Object::new(original_id, kind)
            };

            let mod_amount = source.read_u32::<LE>()?;
            for _ in 0..mod_amount {
                read_field(source, &mut object, is_type_with_data(kind))?;
            }

            objects.insert_object(object);
        }

        Ok(objects)
    }

    /// Reads the given object file, and produces
    /// an `ObjectStore` object containing all read
    /// objects.
    pub fn read_object_file(
        mut source: &[u8],
        kind: ObjectKind,
    ) -> Result<ObjectStore, ObjParseError> {
        // skip version
        source.read_u32::<LE>()?;

        let original = read_object_table(&mut source, kind)?;
        let new = read_object_table(&mut source, kind)?;

        Ok(new.merge_with(original))
    }
}

pub mod write {
    use std::io::{Error as IoError, Write};
    use std::cell::Ref;

    use byteorder::{BE, LE, WriteBytesExt};

    use crate::{ObjectId, ObjectKind};
    use crate::object::{FieldKind, Object, ObjectStore, Value};

    use super::is_type_with_data;

    const W3OBJ_FORMAT_VERSION: u32 = 1;

    type FlatFieldItem<'a> = (ObjectId, u8, u32, &'a Value);

    fn object_flat_fields_data(object: &Object) -> impl Iterator<Item = FlatFieldItem> {
        object.fields().flat_map(|(id, field)| {
            let a: Box<dyn Iterator<Item = FlatFieldItem>> = match &field.kind {
                FieldKind::Simple { value } => Box::new(std::iter::once((*id, 0, 0, value))),
                FieldKind::Data { values } => Box::new(
                    values
                        .iter()
                        .map(move |value| (*id, value.data_id, value.level, &value.value)),
                ),
            };

            a
        })
    }

    fn object_flat_fields_simple(object: &Object) -> impl Iterator<Item = (ObjectId, &Value)> {
        object
            .fields()
            .filter_map(move |(id, field)| match &field.kind {
                FieldKind::Simple { value } => Some((*id, value)),
                FieldKind::Data { .. } => {
                    eprintln!(
                        "unexpected data field in object {} for field {}",
                        object.id(),
                        field.id
                    );
                    None
                }
            })
    }

    fn is_obj_kind_pred(kind: ObjectKind) -> impl Fn(&Ref<Object>) -> bool {
        move |o| o.kind().contains(kind)
    }

    fn is_obj_stock_pred() -> impl Fn(&Ref<Object>) -> bool {
        |o| o.parent_id().is_none()
    }

    fn is_obj_custom_pred() -> impl Fn(&Ref<Object>) -> bool {
        |o| o.parent_id().is_some()
    }

    fn write_string<W: Write>(mut writer: W, string: &str) -> Result<(), IoError> {
        for c in string.as_bytes() {
            if *c == 0 {
                break;
            }

            writer.write_u8(*c)?;
        }
        writer.write_u8(0)?;

        Ok(())
    }

    fn write_value<W: Write>(mut writer: W, value: &Value) -> Result<(), IoError> {
        match value {
            Value::Int(num) => writer.write_i32::<LE>(*num)?,
            Value::Real(num) => writer.write_f32::<LE>(*num)?,
            Value::Unreal(num) => writer.write_f32::<LE>(*num)?,
            Value::String(val) => write_string(&mut writer, val)?,
        }

        Ok(())
    }

    fn write_simple_fields<W: Write>(mut writer: W, object: &Object) -> Result<(), IoError> {
        let fields: Vec<_> = object_flat_fields_simple(object).collect();

        writer.write_u32::<LE>(fields.len() as u32)?;
        for (id, value) in fields {
            writer.write_u32::<BE>(id.to_u32())?;
            writer.write_u32::<LE>(value.type_id())?;

            write_value(&mut writer, value)?;
            writer.write_u32::<BE>(object.id().to_u32())?;
        }

        Ok(())
    }

    fn write_data_fields<W: Write>(mut writer: W, object: &Object) -> Result<(), IoError> {
        let fields: Vec<_> = object_flat_fields_data(object).collect();

        writer.write_u32::<LE>(fields.len() as u32)?;
        for (id, data_id, level, value) in fields {
            writer.write_u32::<BE>(id.to_u32())?;
            writer.write_u32::<LE>(value.type_id())?;
            writer.write_u32::<LE>(level)?;
            writer.write_u32::<LE>(data_id as u32)?;

            write_value(&mut writer, value)?;
            writer.write_u32::<BE>(object.id().to_u32())?;
        }

        Ok(())
    }

    pub fn write_object_file<W: Write>(
        mut writer: W,
        data: &ObjectStore,
        kind: ObjectKind,
    ) -> Result<(), IoError> {
        writer.write_u32::<LE>(W3OBJ_FORMAT_VERSION)?;

        let stock_objs: Vec<_> = data
            .objects()
            .map(|o| o.borrow())
            .filter(is_obj_kind_pred(kind))
            .filter(is_obj_stock_pred())
            .collect();

        let custom_objs: Vec<_> = data
            .objects()
            .map(|o| o.borrow())
            .filter(is_obj_kind_pred(kind))
            .filter(is_obj_custom_pred())
            .collect();

        // write stock objects
        writer.write_u32::<LE>(stock_objs.len() as u32)?;
        for object in stock_objs {
            writer.write_u32::<BE>(object.id().to_u32())?;
            writer.write_u32::<BE>(0)?;

            if is_type_with_data(kind) {
                write_data_fields(&mut writer, &object)?;
            } else {
                write_simple_fields(&mut writer, &object)?;
            }
        }

        // write custom objects
        writer.write_u32::<LE>(custom_objs.len() as u32)?;
        for object in custom_objs {
            writer.write_u32::<BE>(object.parent_id().unwrap().to_u32())?;
            writer.write_u32::<BE>(object.id().to_u32())?;

            if is_type_with_data(kind) {
                write_data_fields(&mut writer, &object)?;
            } else {
                write_simple_fields(&mut writer, &object)?;
            }
        }

        Ok(())
    }
}
