use err_derive::Error;

use std::path::Path;
use std::path::PathBuf;
use std::error::Error;
use std::sync::Arc;

pub type AnyError = Box<dyn Error + Sync + Send + 'static>;

#[derive(Error, Debug)]
#[error(display = "{}: {}", context, cause)]
pub struct ContextError<E: Error> {
    context: String,
    cause:   E,
}

impl<E: Error> ContextError<E> {
    pub fn new<S: AsRef<str>>(context: S, cause: E) -> Self {
        ContextError {
            context: context.as_ref().into(),
            cause,
        }
    }
}

#[derive(Error, Debug)]
#[error(display = "IO Error [{:?}]: {}", path, cause)]
pub struct IoError {
    path:  PathBuf,
    cause: std::io::Error,
}

impl IoError {
    pub fn new<P: AsRef<Path>>(path: P, cause: std::io::Error) -> Self {
        IoError {
            path: path.as_ref().into(),
            cause,
        }
    }
}
