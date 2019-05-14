use pest::iterators::*;
use pest::RuleType;

use std::fs;
use std::path::Path;

pub fn find_pairs_with_rule<'i, R: RuleType>(
    pair: &Pair<'i, R>,
    rule: R,
) -> impl Iterator<Item = Pair<'i, R>> {
    pair.clone()
        .into_inner()
        .flatten()
        .filter(move |pair| pair.as_rule() == rule)
}

pub fn copy_dir_from_to<P: AsRef<Path>, Q: AsRef<Path>>(
    from: P,
    to: Q,
) -> Result<(), std::io::Error> {
    fs::create_dir_all(&to)?;

    for item in fs::read_dir(from)? {
        let item = item?;

        let file_type = item.file_type()?;

        if file_type.is_file() {
            fs::copy(item.path(), &to)?;
        } else if file_type.is_dir() {
            copy_dir_from_to(
                item.path(),
                to.as_ref().join(item.path().components().last().unwrap()),
            )?;
        }
    }

    Ok(())
}
