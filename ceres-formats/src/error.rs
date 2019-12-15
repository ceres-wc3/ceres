use std::io::Error as IoError;

use thiserror::Error;

use crate::ObjectId;

#[derive(Debug, Error)]
pub enum ObjParseError {
    #[error("Parse error")]
    Io {
        #[from]
        source: IoError,
    },
    #[error("C String is unterminated")]
    UnterminatedString,
    #[error("Unknown field {id}")]
    UnknownField { id: ObjectId },
}

impl ObjParseError {
    pub fn unknown_field(id: ObjectId) -> ObjParseError {
        ObjParseError::UnknownField { id }
    }

    pub fn unterminated_string() -> ObjParseError {
        ObjParseError::UnterminatedString
    }
}
