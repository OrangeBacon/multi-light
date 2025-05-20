use std::path::PathBuf;

use crate::{Config, Error};

impl Config {
    /// Parse a toml string
    pub fn from_toml(
        file_name: impl Into<PathBuf>,
        content: impl AsRef<str>,
    ) -> Result<Self, Error> {
        let file_name = file_name.into();

        let toml = toml::from_str(content.as_ref()).map_err(|err| Error::SerdeToml {
            err: Box::new(err),
            file_name: file_name.clone(),
        })?;

        Ok(Self {
            tree: toml,
            file_name,
        })
    }
}
