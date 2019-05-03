use pest_derive::*;

pub use pest::Parser;

#[derive(Parser)]
#[grammar = "lua.pest"]
pub struct LuaParser;

use pest::iterators::*;
use pest::RuleType;

fn pair_to_string<T: RuleType>(pairs: Pairs<T>, indent: usize) {
    for pair in pairs {
        println!(
            "{} >{:?}: {}",
            "  ".repeat(indent),
            pair.as_rule(),
            pair.as_str().replace("\n", "\\n")
        );
        pair_to_string(pair.into_inner(), indent + 1);
    }
}

#[cfg(test)]
mod test {
    use super::pair_to_string;
    use super::LuaParser;
    use super::Rule;
    use pest::Parser;

    // #[test]
    fn test_name_parsing() {
        assert!(LuaParser::parse(Rule::Ident, "test").unwrap().as_str() == "test");
        assert!(LuaParser::parse(Rule::Ident, "_test").unwrap().as_str() == "_test");
        assert!(LuaParser::parse(Rule::Ident, "_1ok").unwrap().as_str() == "_1ok");
        assert!(LuaParser::parse(Rule::Ident, "лул").unwrap().as_str() == "лул");
        assert!(LuaParser::parse(Rule::Ident, "a b").unwrap().as_str() == "a");

        assert!(LuaParser::parse(Rule::Ident, " a").is_err());
        assert!(LuaParser::parse(Rule::Ident, "1лул").is_err());
        assert!(LuaParser::parse(Rule::Ident, "1ok").is_err());
        assert!(LuaParser::parse(Rule::Ident, "false").is_err());
        assert!(LuaParser::parse(Rule::Ident, "true").is_err());
    }

    // #[test]
    fn test_functioncall_parsing() {
        // let mut a = LuaParser::parse(Rule::StmtFuncCall, "hello ()").unwrap();

        // pair_to_string(a.next().unwrap(), 0);

        let a = LuaParser::parse(Rule::StmtFuncCall, "hello[nil]()").unwrap();

        pair_to_string(a, 0);

        LuaParser::parse(Rule::StmtFuncCall, "hello.a.b.c().d.e.f()").unwrap();
        LuaParser::parse(Rule::StmtFuncCall, "hello().a().b().c()").unwrap();
        LuaParser::parse(Rule::StmtFuncCall, "hello().a(hello.b.c.d()).b().c()").unwrap();
    }

    // #[test]
    fn test_full() {
        let src = include_str!("test-cases/simple.lua");

        let out = LuaParser::parse(Rule::Chunk, src).unwrap();

        pair_to_string(out, 0);
    }
}
