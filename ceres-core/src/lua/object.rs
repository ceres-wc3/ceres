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
) -> Result<Value, LuaError> {
    Ok(match field_meta.value_ty {
        ValueType::String => Value::String(FromLua::from_lua(value, ctx)?),
        ValueType::Int => Value::Int(FromLua::from_lua(value, ctx)?),
        ValueType::Real => Value::Real(FromLua::from_lua(value, ctx)?),
        ValueType::Unreal => Value::Unreal(FromLua::from_lua(value, ctx)?),
    })
}

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
                        .into()
                })
        }

        fn get_field_for<C>(object: &Object, field_getter: C) -> Option<&Value>
        where
            C: Fn(&Object) -> Option<&Value>,
        {
            field_getter(object).or_else(|| {
                w3data::data()
                    .object_prototype(&object)
                    .and_then(|proto| field_getter(proto))
            })
        }

        methods.add_method("getId", |ctx, data, _: ()| Ok(todo!()));

        methods.add_method("getParentId", |ctx, data, _: ()| Ok(todo!()));

        methods.add_method("isCustom", |ctx, data, _: ()| Ok(todo!()));

        methods.add_method("getType", |ctx, data, _: ()| Ok(todo!()));

        methods.add_method("clone", |ctx, data, _: ()| Ok(todo!()));

        methods.add_method("getFields", |ctx, object, (): ()| {
            let fields: Vec<String> = w3data::metadata()
                .query_all_object_fields(&object.inner.borrow())
                .map(|desc| {
                    desc.id
                        .to_string()
                        .expect("builtin ID is not representible as a string... wut")
                })
                .collect();

            Ok(fields)
        });

        methods.add_method(
            "setField",
            |ctx, object, (id, value): (LuaValue, LuaValue)| {
                let id = lvalue_to_objid(id)?;
                let mut object = object.inner.borrow_mut();
                let field_meta = get_meta_for(id, &object)?;

                let data_id = match field_meta.variant {
                    FieldVariant::Data { id } => id,
                    _ => 0,
                };

                let value = lvalue_to_value(ctx, value, field_meta)?;

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
                let id = lvalue_to_objid(id)?;
                let mut object = object.inner.borrow_mut();
                let field_meta = get_meta_for(id, &object)?;

                let data_id = match field_meta.variant {
                    FieldVariant::Data { id } => id,
                    FieldVariant::Normal { .. } => {
                        return Err(StringError::new(
                            "cannot set level on field {} because it is not leveled",
                        )
                        .into())
                    }
                    _ => 0,
                };

                let value = lvalue_to_value(ctx, value, field_meta)?;

                object.set_data_field(id, level as u32, data_id, value);

                Ok(())
            },
        );

        methods.add_method("getField", |ctx, object, id: LuaValue| {
            let id = lvalue_to_objid(id)?;
            let object = object.inner.borrow();
            let field_meta = get_meta_for(id, &object)?;

            match field_meta.variant {
                FieldVariant::Data { .. } | FieldVariant::Leveled { .. } => {
                    return Err(StringError::new(format!(
                        "tried to get field {} as simple but it's actually a data field",
                        id
                    ))
                    .into())
                }
                _ => {}
            }

            let field = get_field_for(&object, |o| o.simple_field(id));

            Ok(field.map(|f| value_to_lvalue(ctx, f)))
        });

        methods.add_method(
            "getLevelField",
            |ctx, object, (id, level): (LuaValue, LuaInteger)| {
                let id = lvalue_to_objid(id)?;
                let level = level as u32;
                let object = object.inner.borrow();
                let field_meta = get_meta_for(id, &object)?;

                if let FieldVariant::Normal { .. } = field_meta.variant {
                    return Err(StringError::new(format!(
                        "tried to get field {} as data but it's actually a simple field",
                        id
                    ))
                    .into());
                }

                let data_id = match field_meta.variant {
                    FieldVariant::Data { id } => id,
                    _ => 0,
                };

                let field = get_field_for(&object, |o| o.data_field(id, level, data_id));

                Ok(field.map(|f| value_to_lvalue(ctx, f)))
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
            let obj = data.inner.object(lvalue_to_objid(id)?);

            Ok(obj.map(|o| ObjectWrap {
                inner: Rc::clone(o),
            }))
        });

        methods.add_method("getObjects", |ctx, data, _: ()| {
            let objects: Vec<_> = data.inner.objects()
                .map(|obj| Rc::clone(obj))
                .map(|obj| ObjectWrap { inner: obj })
                .collect();

            Ok(objects)
        });

        methods.add_method_mut("addFrom", |ctx, data, other: LuaAnyUserData| {
            let other = other.borrow_mut::<ObjectStoreWrap>()?;

//            data.inner.

            Ok(todo!())
        });

        methods.add_method("writeToString", |ctx, data, _: ()| Ok(todo!()));

        methods.add_method("addObject", |ctx, data, _: ()| Ok(todo!()));

        methods.add_method("removeObject", |ctx, data, _: ()| Ok(todo!()));
    }
}

fn open_store_from_str(data: &[u8], kind: ObjectKind) -> Result<ObjectStoreWrap, AnyError> {
    let object_data = w3obj::read::read_object_file(data, kind)?;

    Ok(ObjectStoreWrap { inner: object_data })
}

pub fn get_create_new_object_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx: LuaContext, (): ()| Ok(todo!()))
        .unwrap()
}

pub fn get_open_store_from_str_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx: LuaContext, (data, ext): (LuaString, LuaString)| {
        let data = data.as_bytes();
        let kind = ObjectKind::from_ext(ext.to_str()?);

        if kind == ObjectKind::empty() {
            return Err(StringError::new(format!(
                "{} is not a valid format",
                ext.to_str().unwrap()
            ))
            .into());
        }

        let result = open_store_from_str(data, kind);

        Ok(wrap_result(ctx, result))
    })
    .unwrap()
}
