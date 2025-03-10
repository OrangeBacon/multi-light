use std::path::PathBuf;

use crate::Error;

use super::Config;

impl Config {
    /// Parse a JSON string
    pub fn from_json(
        file_name: impl Into<PathBuf>,
        content: impl AsRef<str>,
    ) -> Result<Self, Error> {
        let file_name = file_name.into();

        let json = serde_json::from_str(content.as_ref()).map_err(|err| Error::SerdeJson {
            err,
            file_name: file_name.clone(),
        })?;

        Ok(Self::NonDebug {
            tree: json,
            file_name,
        })
    }
}
