/*
    ceres.loadObjects(filename, kind) -> ObjectStore

*/

use std::cell::RefCell;
use std::rc::Rc;

use rlua::prelude::*;

use ceres_formats::ObjectKind;
use ceres_formats::object::{Object, ObjectStore, Value};
use ceres_formats::parser::w3obj;

use crate::error::AnyError;
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

impl LuaUserData for ValueWrap {}

impl LuaUserData for ObjectWrap {
    fn add_methods<'lua, T>(methods: &mut T)
    where
        T: LuaUserDataMethods<'lua, Self>,
    {
        methods.add_method(
            "setSimpleField",
            |_ctx, obj, (id, value): (LuaValue, LuaAnyUserData)| {
                let id = lvalue_to_objid(id).map_err(LuaError::external)?;
                let value = value.borrow::<ValueWrap>()?;

                let mut obj = obj.inner.borrow_mut();
                obj.set_simple_field(id, value.inner.clone());

                Ok(())
            },
        );

        methods.add_method(
            "setLevelField",
            |_ctx, obj, (id, level, value): (LuaValue, LuaInteger, LuaAnyUserData)| {
                let id = lvalue_to_objid(id).map_err(LuaError::external)?;
                let value = value.borrow::<ValueWrap>()?;

                let mut obj = obj.inner.borrow_mut();
                obj.set_level_field(id, level as u32,value.inner.clone());

                Ok(())
            },
        );

        methods.add_method(
            "setDataField",
            |_ctx, obj, (id, level, data, value): (LuaValue, LuaInteger, LuaInteger, LuaAnyUserData)| {
                let id = lvalue_to_objid(id).map_err(LuaError::external)?;
                let value = value.borrow::<ValueWrap>()?;

                let mut obj = obj.inner.borrow_mut();
                obj.set_data_field(id, level as u32, data as u8, value.inner.clone());

                Ok(())
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

fn open_store_from_str_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx: LuaContext, (data, ext): (LuaString, LuaString)| {
        let result = open_store_from_str(data, ext);

        Ok(wrap_result(ctx, result))
    })
    .unwrap()
}

fn value_str(ctx: LuaContext) -> LuaFunction {
    unimplemented!()
}
