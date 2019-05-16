use pest::iterators::*;
use pest::RuleType;

use std::fs;
use std::path::Path;

use failure::{ResultExt, Error};

pub fn find_pairs_with_rule<'i, R: RuleType>(
    pair: &Pair<'i, R>,
    rule: R,
) -> impl Iterator<Item = Pair<'i, R>> {
    pair.clone()
        .into_inner()
        .flatten()
        .filter(move |pair| pair.as_rule() == rule)
}

pub fn copy_dir_from_to<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<(), Error> {
    fs::create_dir_all(&to).with_context(|_| format!("cannot create folder {:?}", to.as_ref()))?;

    for item in fs::read_dir(from)? {
        dbg!(&item);

        let item = item?;

        let file_type = item.file_type()?;

        if file_type.is_file() {
            fs::copy(
                item.path(),
                to.as_ref().join(item.path().file_name().unwrap()),
            )?;
        } else if file_type.is_dir() {
            copy_dir_from_to(
                item.path(),
                to.as_ref().join(item.path().components().last().unwrap()),
            )?;
        }
    }

    Ok(())
}
