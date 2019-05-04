use pest::iterators::*;
use pest::RuleType;

pub fn find_pairs_with_rule<'i, R: RuleType>(
    pair: &Pair<'i, R>,
    rule: R,
) -> impl Iterator<Item = Pair<'i, R>> {
    pair.clone()
        .into_inner()
        .flatten()
        .filter(move |pair| pair.as_rule() == rule)
}
