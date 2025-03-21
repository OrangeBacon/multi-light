use std::path::{Path, PathBuf};

use crate::Error;

use super::{Config, ConfigTree};

impl Config {
    /// Parse a YAML string. YAML with debug info is not supported.
    pub fn from_yaml(
        file_name: impl Into<PathBuf>,
        content: impl AsRef<str>,
    ) -> Result<Self, Error> {
        let file_name = file_name.into();

        let yaml = yaml_rust2::YamlLoader::load_from_str(content.as_ref()).map_err(|err| {
            Error::YAMLError {
                err: err.to_string(),
                file_name: file_name.clone(),
            }
        })?;

        // only accept 1 document within the file, error if there are multiple
        if yaml.len() != 1 {
            return Err(Error::YAMLError {
                err: format!("Expected 1 document, got {}", yaml.len()),
                file_name: file_name.clone(),
            });
        }

        // checked above that the len == 1, so should never panic
        let yaml = yaml.into_iter().next().unwrap();

        let tree = yaml_visitor(yaml, &file_name)?;

        Ok(Self::NonDebug { tree, file_name })
    }
}

/// Convert yaml_rust2 representation into `ConfigTree<()>`.  Unfortunately
/// this cannot just be a serde deserialize as yaml_rust2 doesn't use serde.
fn yaml_visitor(yaml: yaml_rust2::Yaml, file_name: &Path) -> Result<ConfigTree<()>, Error> {
    match yaml {
        yaml_rust2::Yaml::Real(value) => Ok(ConfigTree::String {
            id: Default::default(),
            value,
        }),
        yaml_rust2::Yaml::Integer(value) => Ok(ConfigTree::String {
            id: Default::default(),
            value: value.to_string(),
        }),
        yaml_rust2::Yaml::String(value) => Ok(ConfigTree::String {
            id: Default::default(),
            value,
        }),
        yaml_rust2::Yaml::Boolean(value) => Ok(ConfigTree::Bool {
            id: Default::default(),
            value,
        }),
        yaml_rust2::Yaml::Array(value) => Ok(ConfigTree::Array {
            id: Default::default(),
            value: value
                .into_iter()
                .map(|v| yaml_visitor(v, file_name))
                .collect::<Result<_, _>>()?,
        }),
        yaml_rust2::Yaml::Hash(value) => {
            let value = value
                .into_iter()
                .map(|(k, v)| {
                    // convert scalar values into a key, otherwise the key is
                    // a map or an array, so it could (technically, if rust_yaml2
                    // implemented it) be infinitely recursive, so do not try to
                    // convert into a string key value.
                    let key = match k {
                        yaml_rust2::Yaml::Real(k) => k,
                        yaml_rust2::Yaml::Integer(k) => k.to_string(),
                        yaml_rust2::Yaml::String(k) => k,
                        yaml_rust2::Yaml::Boolean(k) => k.to_string(),
                        _ => {
                            return Err(Error::YAMLError {
                                err: String::from("Unexpected Complex Key in map"),
                                file_name: file_name.to_path_buf(),
                            });
                        }
                    };
                    Ok((key, yaml_visitor(v, file_name)?))
                })
                .collect::<Result<_, _>>()?;

            Ok(ConfigTree::Object {
                id: Default::default(),
                value,
            })
        }
        yaml_rust2::Yaml::Null => Ok(ConfigTree::Null {
            id: Default::default(),
        }),
        // not fully implemented within yaml_rust2 according to its documentation?
        yaml_rust2::Yaml::Alias(_) => Err(Error::YAMLError {
            err: String::from("yaml_rust2 alias not fully implemented"),
            file_name: file_name.to_path_buf(),
        }),
        yaml_rust2::Yaml::BadValue => Err(Error::YAMLError {
            err: String::from("Bad value found"),
            file_name: file_name.to_path_buf(),
        }),
    }
}
