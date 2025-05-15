use bytes::Bytes;

use crate::shared_lib::types::{Chunk, FileMetadata, ServerClientMsg, TextMsg};

use super::errors::DataParsingError;

pub fn serialize_text_msg(msg: TextMsg) -> Result<Bytes, DataParsingError> {
    let msg = ServerClientMsg::Text(msg);
    let serialized = bincode::serialize(&msg)?;
    Ok(Bytes::from(serialized))
}

pub fn serialize_file_chunk(chunk: Chunk) -> Result<Bytes, DataParsingError> {
    let msg = ServerClientMsg::FileChunk(chunk);
    let serialized = bincode::serialize(&msg)?;
    Ok(Bytes::from(serialized))
}

pub fn serialize_file_metadata(data: FileMetadata) -> Result<Bytes, DataParsingError> {
    let msg = ServerClientMsg::FileMetadata(data);
    let serialized = bincode::serialize(&msg)?;
    Ok(Bytes::from(serialized))
}
