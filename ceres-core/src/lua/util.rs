use rlua::prelude::*;
use pest::iterators::Pair;
use ceres_parsers::lua;
use ceres_formats::ObjectId;

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
        LuaValue::String(s) => Some(format!("\"{}\"", s.to_str().unwrap())),
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

pub fn wrap_result<'lua, V>(ctx: LuaContext<'lua>, value: Result<V, AnyError>) -> LuaMultiValue
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

pub fn lvalue_to_objid(value: LuaValue) -> Result<ObjectId, AnyError> {
    Ok(match value {
        LuaValue::String(value) => ObjectId::from_bytes(value.as_bytes())
            .ok_or_else(|| StringError::new("invalid byte sequence for id"))?,
        LuaValue::Integer(value) => ObjectId::new(value as u32),
        _ => Err(StringError::new("cannot coerce type to object id"))?,
    })
}
