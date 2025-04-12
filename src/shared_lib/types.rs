use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
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

pub struct DirectChannel {
    pub tx: mpsc::Sender<Bytes>,
    pub rx: mpsc::Receiver<Bytes>,
}

pub struct RoomChannel {
    pub tx: broadcast::Sender<Bytes>,
    pub rx: broadcast::Receiver<Bytes>,
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

#[derive(Deserialize, Serialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub enum ServerMessage<'a> {
    Text(TextMessage),
    File(Chunk<'a>),
    FileMetadata(FileMetadata),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct FileMetadata {
    pub size: u64,
    pub name: String,
    pub stream_id: Uuid,
    pub to: Channel,
}
