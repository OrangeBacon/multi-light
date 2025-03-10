//! Generic input parsing, allows all input file types to be converted into one
//! central format, so the rest of the code doesn't have to deal with JSON vs
//! plist vs YAML etc.

mod json;

use core::{fmt::Debug, marker::PhantomData};
use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, de::Visitor};

/// Document representation common to JSON/plist/XML/YAML
#[derive(Debug, Clone)]
pub enum Config {
    /// Version of the config that includes location debug information
    Debug {
        /// Contents of the file
        tree: ConfigTree<ConfigNodeID>,

        /// List of source locations for each node.  If the tree was constructed while
        /// storing position information, a node ID can be used as an index into this
        /// list to get the source location of the individual node.
        location: Vec<Location>,

        /// File name of the document
        file_name: PathBuf,
    },

    /// Version of the config that does not include location debug information.
    /// Should (maybe) be faster to work with as the data structure will be
    /// smaller than the one with debug info.
    NonDebug {
        /// Contents of the file
        tree: ConfigTree<()>,

        /// File name of the document
        file_name: PathBuf,
    },
}

/// Identifier for a single node within a parsed document tree, only applies to
/// the tree that it was parsed from
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct ConfigNodeID(pub usize);

/// Raw tree data within a parsed document.  The generic parameter is used for
/// the NodeID, to allow setting it to a zero-size type if debug info is not
/// needed, therefore making the structure smaller.
#[derive(Clone)]
pub enum ConfigTree<T> {
    Null {
        id: T,
    },
    Bool {
        id: T,
        value: bool,
    },
    String {
        id: T,
        value: String,
    },
    Array {
        id: T,
        value: Vec<ConfigTree<T>>,
    },
    Object {
        id: T,
        value: HashMap<String, ConfigTree<T>>,
    },
}

/// Source location for each item within a config file
#[derive(Debug, Clone, Copy)]
pub struct Location {
    /// line number at the start of the item (1 indexed)
    pub line_number: usize,

    /// column number at the start of the item (1 indexed)
    pub column_number: usize,

    /// byte offset into the source file (0 indexed)
    pub byte_number: usize,
}

impl Config {
    /// Get the source location of a given node within a document
    pub fn source_location(&self, node: ConfigNodeID) -> Option<Location> {
        match self {
            Config::Debug { location, .. } => location.get(node.0).copied(),
            Config::NonDebug { .. } => None,
        }
    }
}

/// Serde derive didn't really do what I wanted for deserializing into this format
/// so here is a custom deserializer.  Note that this still doesn't do anything
/// about line numbers, etc, they are not implemented using serde parsers.
impl<'de, T: Default> Deserialize<'de> for ConfigTree<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ConfigTreeVisitor(Default::default()))
    }
}

struct ConfigTreeVisitor<T>(PhantomData<T>);

impl<'de, T: Default> Visitor<'de> for ConfigTreeVisitor<T> {
    type Value = ConfigTree<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("JSON")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_string(v.to_string())
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
        Ok(ConfigTree::String {
            id: Default::default(),
            value: v,
        })
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
        Ok(ConfigTree::Null {
            id: Default::default(),
        })
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
        Ok(ConfigTree::Null {
            id: Default::default(),
        })
    }

    fn visit_seq<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut array = Vec::with_capacity(access.size_hint().unwrap_or(0));

        while let Some(value) = access.next_element()? {
            array.push(value);
        }

        Ok(ConfigTree::Array {
            id: Default::default(),
            value: array,
        })
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut map = HashMap::with_capacity(access.size_hint().unwrap_or(0));

        while let Some((key, value)) = access.next_entry()? {
            map.insert(key, value);
        }

        Ok(ConfigTree::Object {
            id: Default::default(),
            value: map,
        })
    }
}

/// Custom debug impl of the tree which flattens it if debug info is zero-size.
impl<T: Debug> Debug for ConfigTree<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if std::mem::size_of::<T>() == 0 {
            return match self {
                Self::Null { .. } => f.debug_struct("Null").finish(),
                Self::Bool { value, .. } => Debug::fmt(value, f),
                Self::String { value, .. } => Debug::fmt(value, f),
                Self::Array { value, .. } => f.debug_list().entries(value).finish(),
                Self::Object { value, .. } => f.debug_map().entries(value).finish(),
            };
        }

        match self {
            Self::Null { id } => f.debug_struct("Null").field("id", id).finish(),
            Self::Bool { id, value } => f
                .debug_struct("Bool")
                .field("id", id)
                .field("value", value)
                .finish(),
            Self::String { id, value } => f
                .debug_struct("String")
                .field("id", id)
                .field("value", value)
                .finish(),
            Self::Array { id, value } => f
                .debug_struct("Array")
                .field("id", id)
                .field("value", value)
                .finish(),
            Self::Object { id, value } => f
                .debug_struct("Object")
                .field("id", id)
                .field("value", value)
                .finish(),
        }
    }
}
