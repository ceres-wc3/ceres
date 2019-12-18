pub mod parser {
    pub mod slk;
    pub mod crlf;
    pub mod profile;
    pub mod w3obj;
}

pub mod error;
pub mod metadata;
pub mod object;
pub mod uncase;

use serde::{Serialize, Deserialize};
use bitflags::bitflags;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ObjectId {
    id: u32,
}

impl ObjectId {
    pub fn new(id: u32) -> ObjectId {
        ObjectId { id }
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 4 {
            None
        } else {
            let mut value = 0;
            for i in bytes {
                value <<= 8;
                value += u32::from(*i);
            }

            Some(ObjectId { id: value })
        }
    }

    pub fn to_u32(self) -> u32 {
        self.id
    }
}

impl std::fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.id == 0 {
            write!(f, "ObjectID(NULL)")
        } else {
            let bytes = self.id.to_be_bytes();
            let pretty = std::str::from_utf8(&bytes).ok();

            if let Some(pretty) = pretty {
                write!(f, "ObjectID({})", pretty)
            } else {
                write!(f, "ObjectID({})", self.id)
            }
        }
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.id == 0 {
            write!(f, "NULL")
        } else {
            let bytes = self.id.to_be_bytes();
            let pretty = String::from_utf8_lossy(&bytes);

            write!(f, "{}", pretty)
        }
    }
}

impl From<u32> for ObjectId {
    fn from(other: u32) -> Self {
        Self { id: other }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum ValueType {
    Int,
    Real,
    Unreal,
    String
}

impl ValueType {
    pub fn new(input: &str) -> ValueType {
        match input {
            "real" => ValueType::Real,
            "unreal" => ValueType::Unreal,
            "int" | "bool" => ValueType::Int,
            _ => ValueType::String
        }
    }
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct ObjectKind: u32 {
        const ABILITY = 0x1;
        const BUFF = 0x2;
        const DESTRUCTABLE = 0x4;
        const MISC = 0x8;
        const UNIT = 0x10;
        const UPGRADE = 0x20;
        const ITEM = 0x40;
        const DOODAD = 0x80;
    }
}

impl ObjectKind {
    pub fn from_ext(ext: &str) -> ObjectKind {
        match ext {
            "w3u" => ObjectKind::UNIT,
            "w3a" => ObjectKind::ABILITY,
            "w3t" => ObjectKind::ITEM,
            "w3b" => ObjectKind::DESTRUCTABLE,
            "w3d" => ObjectKind::DOODAD,
            "w3h" => ObjectKind::BUFF,
            "w3q" => ObjectKind::UPGRADE,
            _ => ObjectKind::empty(),
        }
    }
}
