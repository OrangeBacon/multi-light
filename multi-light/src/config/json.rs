use std::path::PathBuf;

use crate::Error;

use super::Config;

impl Config {
    /// Parse a JSON string
    // (Note that the vscode version has 2 parsers, one which includes debug info
    // but I cannot be bothered to deal with the debugging versions of any of the
    // input format parsers)
    pub fn from_json(
        file_name: impl Into<PathBuf>,
        content: impl AsRef<str>,
    ) -> Result<Self, Error> {
        let file_name = file_name.into();

        let json = serde_json::from_str(content.as_ref()).map_err(|err| Error::SerdeJson {
            err,
            file_name: file_name.clone(),
        })?;

        Ok(Self {
            tree: json,
            file_name,
        })
    }
}
