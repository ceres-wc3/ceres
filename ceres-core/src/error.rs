use err_derive::Error;
use pest::error::Error as PestError;
use rlua::prelude::LuaError;

use std::path::Path;
use std::path::PathBuf;
use std::error::Error;

use ceres_parsers::lua;

pub type AnyError = Box<dyn Error + Sync + Send + 'static>;

#[derive(Error, Debug)]
#[error(display = "{}", message)]
pub struct StringError {
    message: String,
}

impl StringError {
    pub fn new<S: Into<String>>(message: S) -> StringError {
        StringError {
            message: message.into(),
        }
    }
}

#[derive(Error, Debug)]
#[error(display = "{}: {}", context, cause)]
pub struct ContextError<E: Error> {
    context: String,
    cause:   E,
}

impl<E: Error> ContextError<E> {
    pub fn new<S: Into<String>>(context: S, cause: E) -> Self {
        ContextError {
            context: context.into(),
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

#[derive(Error, Debug)]
#[error(display = "Could not compile file {:?}\nCause: {}", path, cause)]
pub struct FileCompilationError {
    path:  PathBuf,
    cause: CompilerError,
}

impl FileCompilationError {
    pub fn new(path: PathBuf, cause: CompilerError) -> FileCompilationError {
        FileCompilationError { path, cause }
    }
}

#[derive(Error, Debug)]
pub enum CompilerError {
    #[error(display = "Module not found: {}", module_name)]
    ModuleNotFound { module_name: String },
    #[error(display = "Could not parse file:\n{}", error)]
    ParserFailed { error: PestError<lua::Rule> },
    #[error(
        display = "Could not compile module [{}] ({:?}):\n{}",
        module_name,
        module_path,
        error
    )]
    ModuleError {
        module_name: String,
        module_path: PathBuf,
        error:       Box<CompilerError>,
    },
    #[error(display = "Cyclical dependency found involving module {}", module_name)]
    CyclicalDependency { module_name: String },
    #[error(display = "Macro invocation failed: {}\n{}", error, diagnostic)]
    MacroError {
        error:      Box<MacroInvocationError>,
        diagnostic: PestError<lua::Rule>,
    },
}

impl From<PestError<lua::Rule>> for CompilerError {
    fn from(error: PestError<lua::Rule>) -> CompilerError {
        CompilerError::ParserFailed { error }
    }
}

#[derive(Error, Debug)]
pub enum MacroInvocationError {
    #[error(display = "Lua error while invoking macro: {}", error)]
    LuaError { error: LuaError },
    #[error(display = "Error while invoking macro: {}", message)]
    MessageError { message: String },
    #[error(display = "Compiler error while invoking macro: {}", error)]
    CompilerError { error: CompilerError },
}

impl MacroInvocationError {
    pub fn message(message: String) -> MacroInvocationError {
        MacroInvocationError::MessageError { message }
    }
}

impl From<LuaError> for MacroInvocationError {
    fn from(error: LuaError) -> MacroInvocationError {
        MacroInvocationError::LuaError { error }
    }
}

impl From<CompilerError> for MacroInvocationError {
    fn from(error: CompilerError) -> MacroInvocationError {
        MacroInvocationError::CompilerError { error }
    }
}

pub trait ResultExt<T, E> where E: Error {
    fn context<S>(self, context: S) -> Result<T, ContextError<E>> where S: Into<String>;
}

impl<T, E> ResultExt<T, E> for Result<T, E> where E: Error {
    fn context<S>(self, context: S) -> Result<T, ContextError<E>> where S: Into<String> {
        self.map_err(|cause| ContextError::new(context, cause))
    }
}

pub trait OptionExt<T> {
    fn ok_message<S>(self, message: S) -> Result<T, StringError> where S: Into<String>;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_message<S>(self, message: S) -> Result<T, StringError> where S: Into<String> {
        self.ok_or_else(|| StringError::new(message))
    }    
}