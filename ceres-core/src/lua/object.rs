/*
    ceres.loadObjects(filename, kind) -> ObjectStore

*/

use std::cell::RefCell;
use std::rc::Rc;

use rlua::prelude::*;
use atoi::atoi;

use ceres_formats::{ObjectId, ObjectKind, ValueType};
use ceres_formats::metadata::{FieldDesc, FieldVariant};
use ceres_formats::object::{Field, FieldKind, Object, ObjectStore, Value};
use ceres_formats::parser::w3obj;

use crate::error::{AnyError, ContextError, StringError};
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

        methods.add_method("getId", |ctx, object, _: ()| {
            Ok(object.inner.borrow().id().to_string())
        });

        methods.add_method("getParentId", |ctx, object, _: ()| {
            Ok(object
                .inner
                .borrow()
                .parent_id()
                .and_then(|id| id.to_string()))
        });

        methods.add_method("isCustom", |ctx, object, _: ()| {
            Ok(object.inner.borrow().parent_id().is_some())
        });

        methods.add_method("getType", |ctx, object, _: ()| {
            let type_str = object.inner.borrow().kind().to_typestr();

            Ok(type_str)
        });

        methods.add_method("clone", |ctx, object, id: LuaValue| {
            let id = lvalue_to_objid(id)?;
            let object = object.inner.borrow();
            let mut new_object = Object::with_parent(
                id,
                object.parent_id().unwrap_or_else(|| object.id()),
                object.kind(),
            );

            new_object.add_from(&object);

            Ok(ObjectWrap {
                inner: Rc::new(RefCell::new(new_object)),
            })
        });

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

                let value = if let LuaValue::Nil = value {
                    None
                } else {
                    Some(lvalue_to_value(ctx, value, field_meta)?)
                };

                if data_id == 0 {
                    if let Some(value) = value {
                        object.set_simple_field(id, value);
                    } else {
                        object.unset_simple_field(id);
                    }
                } else if let Some(value) = value {
                    object.set_data_field(id, 0, data_id, value);
                } else {
                    object.unset_data_field(id, 0, data_id);
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

                if let LuaValue::Nil = value {
                    object.unset_data_field(id, level as u32, data_id);
                } else {
                    let value = lvalue_to_value(ctx, value, field_meta)?;

                    object.set_data_field(id, level as u32, data_id, value);
                };


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

        methods.add_method("translateField", |ctx, object, field_name: LuaString| {
            let field_bytes = field_name.as_bytes();

            // check if the field is in the form of 'XXXX' or 'XXXX+Y'
            if (field_bytes.len() == 4) || (field_bytes.len() > 5 && field_bytes[4] == b'+') {
                let object_id = ObjectId::from_bytes(&field_bytes[0..4]).unwrap();

                if let Some(field_desc) =
                    w3data::metadata().query_object_field(object_id, &object.inner.borrow())
                {
                    let level = if field_bytes.len() > 5 {
                        atoi::<u32>(&field_bytes[5..])
                    } else {
                        None
                    };

                    let id = field_desc.id.to_string();
                    if (level.is_some() && field_desc.variant.is_leveled())
                        || (level.is_none() && !field_desc.variant.is_leveled())
                    {
                        return Ok((id, level));
                    }
                }
            }

            let (id, level) = w3data::metadata()
                .query_lua_field(&object.inner.borrow(), field_name.to_str()?)
                .map(|(desc, level)| (desc.id.to_string(), level))
                .unwrap_or((None, None));

            Ok((id, level))
        });
    }
}

impl LuaUserData for ObjectStoreWrap {
    fn add_methods<'lua, T>(methods: &mut T)
    where
        T: LuaUserDataMethods<'lua, Self>,
    {
        methods.add_method_mut("getObject", |ctx, data, id: LuaValue| {
            let id = lvalue_to_objid(id)?;
            let object = data.inner.object(id);

            Ok(object
                .map(|o| ObjectWrap {
                    inner: Rc::clone(o),
                })
                .or_else(|| {
                    // try to get a proxy object from the stock object db
                    // if there's one
                    w3data::data().object(id).map(|object| {
                        let object = Object::new(object.id(), object.kind());
                        data.inner.insert_object(object);

                        ObjectWrap {
                            inner: Rc::clone(data.inner.object(id).unwrap()),
                        }
                    })
                }))
        });

        methods.add_method("getObjects", |ctx, data, _: ()| {
            let objects: Vec<_> = data
                .inner
                .objects()
                .map(|object| Rc::clone(object))
                .map(|object| ObjectWrap { inner: object })
                .collect();

            Ok(objects)
        });

        methods.add_method_mut("addFrom", |ctx, data, other: LuaAnyUserData| {
            let other = other.borrow::<ObjectStoreWrap>()?;
            data.inner.add_from(&other.inner);
            Ok(())
        });

        methods.add_method("writeToString", |ctx, data, ext: LuaString| {
            let mut out: Vec<u8> = Default::default();
            w3obj::write::write_object_file(
                &mut out,
                &data.inner,
                ObjectKind::from_ext(ext.to_str()?),
            )
            .map_err(|err| ContextError::new("couldn't write objects to file", err))?;

            Ok(out)
        });

        methods.add_method_mut("addObject", |ctx, data, object: LuaAnyUserData| {
            let object = object.borrow::<ObjectWrap>()?;
            let object = object.inner.borrow().clone();

            data.inner.insert_object(object);

            Ok(())
        });

        methods.add_method_mut("removeObject", |ctx, data, id: LuaValue| {
            let id = lvalue_to_objid(id)?;
            data.inner.remove_object(id);

            Ok(())
        });
    }
}

fn open_store_from_str(data: &[u8], kind: ObjectKind) -> Result<ObjectStoreWrap, AnyError> {
    let object_data = w3obj::read::read_object_file(data, kind)?;

    Ok(ObjectStoreWrap { inner: object_data })
}

fn get_open_store_from_str_luafn(ctx: LuaContext) -> LuaFunction {
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

pub fn get_object_module(ctx: LuaContext) -> LuaTable {
    let table = ctx.create_table().unwrap();

    table
        .set("openStore", get_open_store_from_str_luafn(ctx))
        .unwrap();

    table
}
