use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct InitClientData {
    pub id: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct User {
    // username: String,
    pub id: Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Channel {
    Room(Uuid),
    Direct(Uuid),
}

#[derive(Serialize, Deserialize)]
pub struct TextMessage {
    pub text: String,
    pub from: User,
    pub to: Channel,
}
#[derive(Serialize, Deserialize)]
pub struct Chunk<'a> {
    pub from: User,
    #[serde(with = "serde_bytes")]
    pub data: &'a [u8],
    pub to: Channel,
    pub stream_id: Uuid,
}

// messages both client -> server  and server -> client
#[derive(Deserialize, Serialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub enum ServerMessage<'a> {
    Text(TextMessage),
    FileChunk(Chunk<'a>),
    FileMetadata(FileMetadata),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct FileMetadata {
    pub name: String,
    pub stream_id: Uuid,
    pub to: Channel,
}
