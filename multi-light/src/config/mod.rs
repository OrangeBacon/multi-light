//! Generic input parsing, allows all input file types to be converted into one
//! central format, so the rest of the code doesn't have to deal with JSON vs
//! plist vs YAML etc.

mod json;
mod toml;
mod yaml;

use std::{collections::HashMap, fmt::Debug, path::PathBuf};

use serde::{Deserialize, de::Visitor};

/// Document representation common to JSON/plist/XML/YAML
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Contents of the file
    tree: ConfigTree,

    /// File name of the document
    file_name: PathBuf,
}

/// Identifier for a single node within a parsed document tree, only applies to
/// the tree that it was parsed from
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct ConfigNodeID(pub usize);

/// Raw tree data within a parsed document.  The generic parameter is used for
/// the NodeID, to allow setting it to a zero-size type if debug info is not
/// needed, therefore making the structure smaller.
#[derive(Clone, PartialEq, Eq)]
pub enum ConfigTree {
    Null,
    Bool(bool),
    String(String),
    Array(Vec<ConfigTree>),
    Object(HashMap<String, ConfigTree>),
}

/// Serde derive didn't really do what I wanted for deserializing into this format
/// so here is a custom deserializer.  Note that this still doesn't do anything
/// about line numbers, etc, they are not implemented using serde parsers.
impl<'de> Deserialize<'de> for ConfigTree {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ConfigTreeVisitor)
    }
}

struct ConfigTreeVisitor;

impl<'de> Visitor<'de> for ConfigTreeVisitor {
    type Value = ConfigTree;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("JSON")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ConfigTree::Bool(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_string(v.to_string())
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_string(v.to_string())
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_string(v.to_string())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ConfigTree::String(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_string(v.to_string())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ConfigTree::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ConfigTree::Null)
    }

    fn visit_seq<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut array = Vec::with_capacity(access.size_hint().unwrap_or(0));

        while let Some(value) = access.next_element()? {
            array.push(value);
        }

        Ok(ConfigTree::Array(array))
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut map = HashMap::with_capacity(access.size_hint().unwrap_or(0));

        while let Some((key, value)) = access.next_entry()? {
            map.insert(key, value);
        }

        Ok(ConfigTree::Object(map))
    }
}

/// Custom debug impl of the tree which flattens it if debug info is zero-size.
impl Debug for ConfigTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => f.debug_struct("Null").finish(),
            Self::Bool(value) => Debug::fmt(value, f),
            Self::String(value) => Debug::fmt(value, f),
            Self::Array(value) => f.debug_list().entries(value).finish(),
            Self::Object(value) => f.debug_map().entries(value).finish(),
        }
    }
}
