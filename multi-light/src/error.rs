use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    SerdeJson {
        err: serde_json::Error,
        file_name: PathBuf,
    },
}
