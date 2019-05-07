use pest_derive::*;

pub use pest::Parser;

#[derive(Parser)]
#[grammar = "lua.pest"]
pub struct LuaParser;

#[cfg(test)]
mod test {
    use super::LuaParser;
    use super::Rule;
    use pest::Parser;

    #[test]
    fn lua_test_suite() {
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/all.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/api.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/attrib.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/big.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/bitwise.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/calls.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/closure.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/code.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/constructs.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/coroutine.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/db.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/errors.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/events.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/files.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/gc.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/goto.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/literals.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/locals.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/main.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/math.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/nextvar.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/pm.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/sort.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/strings.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/tpack.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/utf8.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/vararg.lua")).unwrap();
        LuaParser::parse(Rule::Chunk, include_str!("test-cases/verybig.lua")).unwrap();
    }
}
