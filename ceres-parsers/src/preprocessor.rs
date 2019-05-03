use pest_derive::*;

#[derive(Parser)]
#[grammar = "preprocessor.pest"]
pub struct PreprocessorParser;

#[cfg(test)]
mod test {
    use super::PreprocessorParser;
    use super::Rule;
    use pest::iterators::*;
    use pest::Parser;
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

    #[test]
    fn test() {
        let src = include_str!("test-cases/simple.lua");

        let out = PreprocessorParser::parse(Rule::File, src).unwrap();

        println!("");
        pair_to_string(out, 0);
    }
}
