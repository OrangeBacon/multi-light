use std::{fmt::Display, path::PathBuf};

/// Generic errors that can be thrown by the library.
#[derive(Debug)]
pub enum Error {
    SerdeJson {
        err: serde_json::Error,
        file_name: PathBuf,
    },
    JSONError {
        err: String,
        file_name: PathBuf,
    },
    YAMLError {
        err: String,
        file_name: PathBuf,
    },
    SerdeToml {
        err: Box<toml::de::Error>,
        file_name: PathBuf,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SerdeJson { err, file_name } => writeln!(
                f,
                "Error while parsing JSON file `{}`: {err}",
                file_name.display()
            ),
            Error::JSONError { err, file_name } => writeln!(
                f,
                "Error while parsing JSON file `{}`: {err}",
                file_name.display()
            ),
            Error::YAMLError { err, file_name } => writeln!(
                f,
                "Error while parsing YAML file `{}`: {err}",
                file_name.display()
            ),
            Error::SerdeToml { err, file_name } => writeln!(
                f,
                "Error while parsing TOML file `{}`: {err}",
                file_name.display()
            ),
        }
    }
}
