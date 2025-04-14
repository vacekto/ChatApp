use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct InitClientData {
    pub id: Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    // username: String,
    pub id: Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Channel {
    Room(Uuid),
    Direct(Uuid),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TextMessage {
    pub text: String,
    pub from: User,
    pub to: Channel,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Chunk {
    pub from: User,
    #[serde(with = "serde_bytes")]
    pub data: [u8; 8192],
    pub to: Channel,
    pub stream_id: Uuid,
}

// messages both client -> server  and server -> client
#[derive(Deserialize, Serialize, Debug)]
pub enum ServerMessage {
    Text(TextMessage),
    FileChunk(Chunk),
    FileMetadata(FileMetadata),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct FileMetadata {
    pub name: String,
    pub stream_id: Uuid,
    pub to: Channel,
    pub size: u64,
}
