pub mod read {
    use byteorder::{ReadBytesExt, LE, BE};

    use crate::object::{ObjectStore, Object, Field, Value};
    use crate::{ObjectId, ObjectKind};
    use crate::error::*;

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
            3 => Value::Str(read_str(source).map(|s| String::from_utf8_lossy(s).into_owned())?),
            _ => panic!("malformed"),
        })
    }

    fn read_field(
        source: &mut &[u8],
        uses_extra_ints: bool,
    ) -> Result<(ObjectId, Field), ObjParseError> {
        let field_id = source.read_u32::<BE>().map(ObjectId::new)?;
        let field_type = source.read_u32::<LE>()?;

        let field = if !uses_extra_ints {
            let value = read_value(source, field_type)?;

            Field::simple(value)
        } else {
            let level = source.read_u32::<LE>()?;
            let data_id = source.read_u32::<LE>()?;
            let value = read_value(source, field_type)?;

            Field::data(value, (data_id) as u8, level)
        };

        // read trailing int
        source.read_u32::<LE>()?;

        Ok((field_id, field))
    }

    fn is_type_with_data(kind: ObjectKind) -> bool {
        match kind {
            ObjectKind::DOODAD | ObjectKind::ABILITY | ObjectKind::UPGRADE => true,
            _ => false,
        }
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
                let result = read_field(source, is_type_with_data(kind));

                match result {
                    Err(err) => {
                        return Err(err);
                    }
                    Ok((field_id, field)) => {
                        object.set_field(field_id, field);
                    }
                }
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
