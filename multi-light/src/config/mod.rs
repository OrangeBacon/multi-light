//! Generic input parsing, allows all input file types to be converted into one
//! central format, so the rest of the code doesn't have to deal with JSON vs
//! plist vs YAML etc.

use std::{collections::HashMap, path::PathBuf};

use crate::Error;

/// Document representation common to JSON/plist/XML/YAML
pub struct ParsedConfig {
    /// Contents of the file
    pub tree: ParsedConfigTree,

    /// List of source locations for each node.  If the tree was constructed while
    /// storing position information, a node ID can be used as an index into this
    /// list to get the source location of the individual node.
    pub location: Vec<Location>,

    /// File name of the document
    pub file_name: PathBuf,
}

/// Identifier for a single node within a parsed document tree, only applies to
/// the tree that it was parsed from
pub struct ParsedConfigNodeID(pub usize);

/// Raw tree data within a parsed document
pub enum ParsedConfigTree {
    Null {
        id: ParsedConfigNodeID,
    },
    Bool {
        id: ParsedConfigNodeID,
        value: bool,
    },
    String {
        id: ParsedConfigNodeID,
        value: String,
    },
    Array {
        id: ParsedConfigNodeID,
        value: Vec<ParsedConfig>,
    },
    Object {
        id: ParsedConfigNodeID,
        value: HashMap<String, ParsedConfig>,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct Location {
    pub line_number: usize,
    pub column_number: usize,
    pub byte_number: usize,
}

impl ParsedConfig {
    /// Get the source location of a given node within a document
    pub fn source_location(&self, node: ParsedConfigNodeID) -> Option<Location> {
        self.location.get(node.0).copied()
    }

    /// Parse a JSON string
    pub fn from_json(file_name: impl Into<PathBuf>) -> Result<Self, Error> {
        Ok(Self {
            tree: ParsedConfigTree::Null {
                id: ParsedConfigNodeID(0),
            },
            location: vec![],
            file_name: file_name.into(),
        })
    }
}
