use rlua::prelude::*;
use serde::{Deserialize, Serialize};

use anyhow::anyhow;
use bitflags::bitflags;

pub mod parser {
    pub mod slk;
    pub mod crlf;
    pub mod profile;
    pub mod w3obj;
}

pub mod error;
pub mod metadata;
pub mod object;
pub mod objectstore;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
/// A WC3 object id, which is conceptually a simple 32-bit integer,
/// but often represented as a 4-char ASCII string.
///
/// Provides conversion to/from byte arrays for this reason.
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

    pub fn to_string(self) -> Option<String> {
        let bytes: Vec<u8> = (&self.id.to_be_bytes()).iter().copied().collect();
        String::from_utf8(bytes).ok()
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

impl<'lua> FromLua<'lua> for ObjectId {
    fn from_lua(value: LuaValue<'lua>, _ctx: LuaContext<'lua>) -> Result<Self, LuaError> {
        match value {
            LuaValue::String(value) => ObjectId::from_bytes(value.as_bytes()).ok_or_else(|| {
                LuaError::FromLuaConversionError {
                    from:    "string",
                    to:      "objectid",
                    message: Some("invalid byte sequence for object id".into()),
                }
            }),
            LuaValue::Integer(value) => Ok(ObjectId::new(value as u32)),
            _ => Err(LuaError::external(anyhow!(
                "only strings and integers can be converted to object ids"
            ))),
        }
    }
}

impl<'lua> ToLua<'lua> for ObjectId {
    fn to_lua(self, ctx: LuaContext<'lua>) -> Result<LuaValue<'lua>, LuaError> {
        if let Some(value) = self.to_string() {
            Ok(LuaValue::String(ctx.create_string(&value)?))
        } else {
            Ok(LuaValue::Integer(self.id as i64))
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
/// Represents a WC3 primitive data type.
///
/// WC3 field metadata specifies many more types than these,
/// but most of them collapse to strings.
pub enum ValueType {
    Int,
    Real,
    Unreal,
    String,
}

impl ValueType {
    /// Collapse a WC3 data type into a primitive value type.
    ///
    /// Mostly supposed to be used with data types specified in SLKs.
    pub fn new(input: &str) -> ValueType {
        match input {
            "real" => ValueType::Real,
            "unreal" => ValueType::Unreal,
            "int" | "bool" | "attackBits" | "deathType" | "defenseTypeInt" | "detectionType"
            | "teamColor" | "morphFlags" | "silenceFlags" | "stackFlags" | "interactionFlags"
            | "pickFlags" | "versionFlags" | "fullFlags" | "channelType" | "channelFlags"
            | "spellDetail" | "techAvail" => ValueType::Int,
            _ => ValueType::String,
        }
    }
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    /// Represents a WC3 object type.
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
    /// Converts an extension of a WC3 object data file
    /// to its corresponding object type.
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

    pub fn to_ext(self) -> &'static str {
        match self {
            ObjectKind::UNIT => "w3u",
            ObjectKind::ABILITY => "w3a",
            ObjectKind::ITEM => "w3t",
            ObjectKind::DESTRUCTABLE => "w3b",
            ObjectKind::DOODAD => "w3d",
            ObjectKind::BUFF => "w3h",
            ObjectKind::UPGRADE => "w3q",
            _ => "none",
        }
    }

    /// Returns true if the object type is capable
    /// of using data/leveled fields instead of just regular fields.
    ///
    /// This affects the layout of WC3 object data files.
    pub fn is_data_type(self) -> bool {
        match self {
            ObjectKind::DOODAD | ObjectKind::ABILITY | ObjectKind::UPGRADE => true,
            _ => false,
        }
    }

    pub fn to_typestr(self) -> &'static str {
        match self {
            ObjectKind::UNIT => "unit",
            ObjectKind::ABILITY => "ability",
            ObjectKind::ITEM => "item",
            ObjectKind::DESTRUCTABLE => "destructable",
            ObjectKind::DOODAD => "doodad",
            ObjectKind::BUFF => "buff",
            ObjectKind::UPGRADE => "upgrade",
            _ => "none",
        }
    }
}
