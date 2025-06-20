use std::collections::HashMap;

use crate::{Config, Error};

/// Storage for all data required to syntax highlight a piece of source code
pub struct Registry {
    /// function to use to read a file referenced from a source file
    callback: Option<Box<dyn Fn()>>,

    themes: HashMap<String, Config>,
}

impl Registry {
    /// Create a new registry, containing no syntaxes or grammars
    pub fn new() -> Self {
        Registry {
            callback: None,
            themes: HashMap::new(),
        }
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    // fn read_file(f: fn(&str) -> Result<&str>) {} // function for reading a required dependency file

    /// Add a  new file to the registry
    pub fn add(&mut self, name: &str, input: &str) -> Result<(), Error> {
        let cfg = Config::from_plist(name, input)
            .or_else(|_| Config::from_json(name, input))
            .or_else(|_| Config::from_toml(name, input))
            .or_else(|_| Config::from_yaml(name, input))?;

        self.themes.insert(name.to_string(), cfg);

        Ok(())
    }

    // // Get the theme for a given name (or default if there isn't one already).  Allows for more complex construction of themes, i.e.
    // // if you want to merge them, read them, modify them based on code, etc. (do the same for grammars)
    // fn theme(name: &str) -> Theme<'a> {}
    // fn syntax(name: &str) -> Syntax<'a> {}

    // // parse the document.  If using the mut parser, allow calling the read_file callback, otherwise the registry cannot
    // // be changed, so return an error if a file is not found.
    // // allow for language detection based on the available grammars, or, provide a language name
    // fn parse(&self, input: &str) -> output {}
    // fn parse_mut(&mut self, input: &str) -> output {}
}
