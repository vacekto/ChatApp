use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
pub struct FileMetadata {
    filename: String,
    file_length: usize,
}
