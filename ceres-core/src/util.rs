use matches::matches;
use pest::iterators::*;
use pest::Parser;
use pest::RuleType;
use rlua::Lua;

use ceres_parsers::lua;

pub fn find_pairs_with_rule<'i, R: RuleType>(
    pair: &Pair<'i, R>,
    rule: R,
) -> impl Iterator<Item = Pair<'i, R>> {
    pair.clone()
        .into_inner()
        .flatten()
        .filter(move |pair| pair.as_rule() == rule)
}
