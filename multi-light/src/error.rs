use std::{fmt::Display, path::PathBuf};

/// Generic errors that can be thrown by the library.
#[derive(Debug)]
pub enum Error {
    SerdeJson {
        err: serde_json::Error,
        file_name: PathBuf,
    },
    YAMLScanError {
        err: yaml_rust2::ScanError,
        file_name: PathBuf,
    },
    YAMLDocumentCount {
        count: usize,
        file_name: PathBuf,
    },
    YAMLInvalid {
        file_name: PathBuf,
    },
    YAMLComplexKey {
        file_name: PathBuf,
    },
    YAMLAlias {
        file_name: PathBuf,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SerdeJson { err, file_name } => {
                writeln!(
                    f,
                    "Error while parsing JSON file `{}`: {err}",
                    file_name.display()
                )
            }
            Error::YAMLScanError { err, file_name } => {
                writeln!(
                    f,
                    "Error while parsing YAML file `{}`: {err}",
                    file_name.display()
                )
            }
            Error::YAMLDocumentCount { count, file_name } => {
                writeln!(
                    f,
                    "Error while parsing YAML file `{}`: expected 1 YAML document, got {count}",
                    file_name.display()
                )
            }
            Error::YAMLInvalid { file_name } => {
                writeln!(f, "Bad YAML value within file `{}`", file_name.display())
            }
            Error::YAMLComplexKey { file_name } => {
                writeln!(
                    f,
                    "Non-string map key found within YAML file `{}`",
                    file_name.display()
                )
            }
            Error::YAMLAlias { file_name } => {
                writeln!(
                    f,
                    "YAML file parser emitted alias from file `{}` (yaml_rust2 shows alias as TODO)",
                    file_name.display()
                )
            }
        }
    }
}
