use std::borrow::Borrow;

use serde::{Serialize, Deserialize};
use unicase::UniCase;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct Uncase {
    #[serde(with = "unicase_serde::unicase")]
    inner: UniCase<String>,
}

impl Uncase {
    pub fn new(s: String) -> Uncase {
        Uncase {
            inner: UniCase::new(s),
        }
    }

    pub fn into_inner(self) -> UniCase<String> {
        self.inner
    }
}

impl AsRef<UniCase<String>> for Uncase {
    fn as_ref(&self) -> &UniCase<String> {
        &self.inner
    }
}

impl AsRef<str> for Uncase {
    fn as_ref(&self) -> &str {
        self.inner.as_ref()
    }
}

impl Borrow<str> for Uncase {
    fn borrow(&self) -> &str {
        self.inner.borrow()
    }
}

impl AsMut<UniCase<String>> for Uncase {
    fn as_mut(&mut self) -> &mut UniCase<String> {
        &mut self.inner
    }
}
