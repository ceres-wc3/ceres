/*
    ceres.loadObjects(filename, kind) -> ObjectStore

*/

use std::cell::RefCell;
use std::rc::Rc;

use rlua::prelude::*;

use ceres_formats::{ObjectKind, ObjectId, ValueType};
use ceres_formats::object::{Object, ObjectStore, Field, FieldKind, Value};
use ceres_formats::parser::w3obj;
use ceres_formats::metadata::{FieldDesc, FieldVariant};

use crate::error::{AnyError, StringError};
use crate::lua::util::*;

struct ValueWrap {
    inner: Value,
}

struct ObjectWrap {
    inner: Rc<RefCell<Object>>,
}

struct ObjectStoreWrap {
    inner: ObjectStore,
}

fn value_to_lvalue<'lua>(ctx: LuaContext<'lua>, value: &Value) -> LuaValue<'lua> {
    match value {
        Value::Unreal(value) | Value::Real(value) => LuaValue::Number(*value as LuaNumber),
        Value::Int(value) => LuaValue::Integer(*value as LuaInteger),
        Value::String(value) => LuaValue::String(ctx.create_string(value).unwrap()),
    }
}

fn lvalue_to_value<'lua>(
    ctx: LuaContext<'lua>,
    value: LuaValue<'lua>,
    field_meta: &FieldDesc,
) -> Result<Value, AnyError> {
    Ok(match field_meta.value_ty {
        ValueType::String => Value::String(FromLua::from_lua(value, ctx)?),
        ValueType::Int => Value::Int(FromLua::from_lua(value, ctx)?),
        ValueType::Real => Value::Real(FromLua::from_lua(value, ctx)?),
        ValueType::Unreal => Value::Unreal(FromLua::from_lua(value, ctx)?),
    })
}

impl LuaUserData for ValueWrap {}

impl LuaUserData for ObjectWrap {
    fn add_methods<'lua, T>(methods: &mut T)
    where
        T: LuaUserDataMethods<'lua, Self>,
    {
        fn get_meta_for(id: ObjectId, object: &Object) -> Result<&'static FieldDesc, LuaError> {
            w3data::metadata()
                .query_object_field(id, &object)
                .ok_or_else(|| {
                    StringError::new(format!("no such field {} on object {}", id, object.id()))
                })
                .map_err(LuaError::external)
        }

        methods.add_method(
            "setField",
            |ctx, object, (id, value): (LuaValue, LuaValue)| {
                let id = lvalue_to_objid(id).map_err(LuaError::external)?;
                let mut object = object.inner.borrow_mut();
                let field_meta = get_meta_for(id, &object)?;

                let data_id = match field_meta.variant {
                    FieldVariant::Data { id } => id,
                    _ => 0,
                };

                let value = lvalue_to_value(ctx, value, field_meta).map_err(LuaError::external)?;

                if data_id == 0 {
                    object.set_simple_field(id, value);
                } else {
                    object.set_data_field(id, 0, data_id, value);
                }

                Ok(())
            },
        );

        methods.add_method(
            "setLevelField",
            |ctx, object, (id, level, value): (LuaValue, LuaInteger, LuaValue)| {
                let id = lvalue_to_objid(id).map_err(LuaError::external)?;
                let mut object = object.inner.borrow_mut();
                let field_meta = get_meta_for(id, &object)?;

                // we don't take the data id from the user because the data id field
                // is basically unused... but still a requirement
                // we can get it from the metadata instead
                let data_id = match field_meta.variant {
                    FieldVariant::Data { id } => id,
                    _ => 0,
                };

                let value = lvalue_to_value(ctx, value, field_meta).map_err(LuaError::external)?;

                object.set_data_field(id, level as u32, data_id, value);

                Ok(())
            },
        );

        methods.add_method("getField", |ctx, object, id: LuaValue| {
            let id = lvalue_to_objid(id).map_err(LuaError::external)?;
            let object = object.inner.borrow();
            let field_meta = get_meta_for(id, &object)?;

            match field_meta.variant {
                FieldVariant::Data { .. } | FieldVariant::Leveled { .. } => {
                    return Err(LuaError::external(StringError::new(format!(
                        "tried to get field {} as simple but it's actually a data field",
                        id
                    ))))
                }
                _ => {}
            }

            let field = object
                .simple_field(id)
                .or_else(|| {
                    w3data::data()
                        .object_prototype(&object)
                        .and_then(|proto| proto.simple_field(id))
                })
                .ok_or_else(|| {
                    StringError::new(format!("no such field {} on object {}", id, object.id()))
                })
                .map_err(LuaError::external)?;

            Ok(value_to_lvalue(ctx, field))
        });

        methods.add_method(
            "getLevelField",
            |ctx, object, (id, level): (LuaValue, LuaInteger)| {
                let id = lvalue_to_objid(id).map_err(LuaError::external)?;
                let level = level as u32;
                let object = object.inner.borrow();
                let field_meta = get_meta_for(id, &object)?;

                if let FieldVariant::Normal { .. } = field_meta.variant {
                    return Err(LuaError::external(StringError::new(format!(
                        "tried to get field {} as data but it's actually a simple field",
                        id
                    ))));
                }

                let data_id = match field_meta.variant {
                    FieldVariant::Data { id } => id,
                    _ => 0,
                };

                let field = object
                    .data_field(id, level, data_id)
                    .or_else(|| {
                        w3data::data()
                            .object_prototype(&object)
                            .and_then(|proto| proto.data_field(id, level, data_id))
                    })
                    .ok_or_else(|| {
                        StringError::new(format!("no such field {} on object {}", id, object.id()))
                    })
                    .map_err(LuaError::external)?;

                Ok(value_to_lvalue(ctx, field))
            },
        );
    }
}

impl LuaUserData for ObjectStoreWrap {
    fn add_methods<'lua, T>(methods: &mut T)
    where
        T: LuaUserDataMethods<'lua, Self>,
    {
        methods.add_method("getObject", |ctx, data, id: LuaValue| {
            let obj = data
                .inner
                .object(lvalue_to_objid(id).map_err(LuaError::external)?);

            Ok(obj.map(|o| ObjectWrap {
                inner: Rc::clone(o),
            }))
        });
    }
}

fn open_store_from_str(input: LuaString, ext: LuaString) -> Result<ObjectStoreWrap, AnyError> {
    let data = input.as_bytes();
    let kind = ObjectKind::from_ext(ext.to_str()?);

    let object_data = w3obj::read::read_object_file(data, kind)?;

    Ok(ObjectStoreWrap { inner: object_data })
}

pub fn get_open_store_from_str_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx: LuaContext, (data, ext): (LuaString, LuaString)| {
        let result = open_store_from_str(data, ext);

        Ok(wrap_result(ctx, result))
    })
    .unwrap()
}

pub fn get_int_value_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx, value: LuaInteger| {
        Ok(ValueWrap {
            inner: Value::Int(value as i32),
        })
    })
    .unwrap()
}

pub fn get_real_value_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx, value: LuaNumber| {
        Ok(ValueWrap {
            inner: Value::Real(value as f32),
        })
    })
    .unwrap()
}

pub fn get_unreal_value_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx, value: LuaNumber| {
        Ok(ValueWrap {
            inner: Value::Unreal(value.max(1.0).min(0.0) as f32),
        })
    })
    .unwrap()
}

pub fn get_string_value_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx, value: LuaString| {
        Ok(ValueWrap {
            inner: Value::String(value.to_str()?.to_string()),
        })
    })
    .unwrap()
}
