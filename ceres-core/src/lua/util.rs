use pest::iterators::Pair;
use rlua::prelude::*;

use ceres_formats::{ObjectId, ValueType};
use ceres_formats::metadata::FieldDesc;
use ceres_formats::object::Value;
use ceres_parsers::lua;

use crate::error::*;

pub fn evaluate_macro_args<'lua>(
    ctx: LuaContext<'lua>,
    args: Pair<lua::Rule>,
) -> Result<LuaMultiValue<'lua>, LuaError> {
    if let Some(inner) = args.into_inner().next() {
        let chunk = ctx.load(inner.as_str());

        chunk.eval()
    } else {
        Ok(LuaMultiValue::new())
    }
}

pub fn is_value_stringable(value: &LuaValue) -> bool {
    match value {
        LuaValue::Boolean(_) => true,
        LuaValue::String(_) => true,
        LuaValue::Integer(_) => true,
        LuaValue::Number(_) => true,
        LuaValue::Table(_) => true,
        _ => false,
    }
}

pub fn lvalue_to_str(value: LuaValue) -> Option<String> {
    if !is_value_stringable(&value) {
        return None;
    }

    match value {
        LuaValue::Boolean(b) => {
            if b {
                Some("true".into())
            } else {
                Some("false".into())
            }
        }
        LuaValue::String(s) => {
            let s = s.to_str().unwrap().escape_debug().to_string();

            Some(format!("\"{}\"", s))
        }
        LuaValue::Integer(i) => Some(i.to_string()),
        LuaValue::Number(n) => Some(n.to_string()),
        LuaValue::Table(t) => Some(ltable_to_str(t)),

        _ => unreachable!(),
    }
}

pub fn ltable_to_str(table: LuaTable) -> String {
    let mut out = String::new();

    out += "{";

    for kv in table.pairs::<LuaValue, LuaValue>() {
        let (k, v) = kv.unwrap();

        if !is_value_stringable(&k) || !is_value_stringable(&v) {
            continue;
        }

        if let LuaValue::Table(_) = k {
            continue;
        }

        let ks = lvalue_to_str(k).unwrap();
        let vs = lvalue_to_str(v).unwrap();

        out += "[";
        out += &ks;
        out += "] = ";
        out += &vs;
        out += ",";
    }

    out += "}";

    out
}

pub fn wrap_result<'lua, V>(ctx: LuaContext<'lua>, value: Result<V, anyhow::Error>) -> LuaMultiValue
where
    V: ToLuaMulti<'lua>,
{
    match value {
        Ok(value) => value.to_lua_multi(ctx).unwrap(),
        Err(error) => (
            LuaValue::Boolean(false),
            error.to_string().to_lua(ctx).unwrap(),
        )
            .to_lua_multi(ctx)
            .unwrap(),
    }
}

pub fn lvalue_to_objid(value: LuaValue) -> Result<ObjectId, LuaError> {
    match value {
        LuaValue::String(value) => ObjectId::from_bytes(value.as_bytes())
            .ok_or_else(|| StringError::new("invalid byte sequence for id").into()),
        LuaValue::Integer(value) => Ok(ObjectId::new(value as u32)),
        _ => Err(StringError::new("cannot coerce type to object id").into()),
    }
}

pub fn value_to_lvalue<'lua>(ctx: LuaContext<'lua>, value: &Value) -> LuaValue<'lua> {
    match value {
        Value::Unreal(value) | Value::Real(value) => LuaValue::Number(*value as LuaNumber),
        Value::Int(value) => LuaValue::Integer(*value as LuaInteger),
        Value::String(value) => LuaValue::String(ctx.create_string(value).unwrap()),
    }
}

pub fn lvalue_to_value<'lua>(
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
