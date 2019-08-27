use std::error::Error;

use rlua::prelude::*;
use pest::iterators::Pair;
use ceres_parsers::lua;

use crate::error::AnyError;

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

pub fn value_to_string(value: LuaValue) -> Option<String> {
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
        LuaValue::Table(t) => Some(table_to_string(t)),

        _ => unreachable!(),
    }
}

pub fn table_to_string(table: LuaTable) -> String {
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

        let ks = value_to_string(k).unwrap();
        let vs = value_to_string(v).unwrap();

        out += "[";
        out += &ks;
        out += "] = ";
        out += &vs;
        out += ",";
    }

    out += "}";

    out
}

pub fn lua_wrap_result<'lua, V>(
    ctx: LuaContext<'lua>,
    value: Result<V, AnyError>,
) -> (LuaValue, LuaValue)
where
    V: ToLua<'lua>,
{
    match value {
        Ok(value) => (value.to_lua(ctx).unwrap(), LuaValue::Nil),
        Err(error) => (LuaValue::Boolean(false), error.to_string().to_lua(ctx).unwrap()),
    }
}
