use crate::client_lib::util::config::TCP_CHUNK_BUFFER_SIZE;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InitClientData {
    pub id: Uuid,
    pub username: String,
    // pub room_channels: Vec<RoomChannel>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub username: String,
    pub id: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Channel {
    Room(Uuid),
    User(Uuid),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TextMsg {
    pub text: String,
    pub from: User,
    pub to: Channel,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Chunk {
    pub from: User,
    #[serde(with = "serde_bytes")]
    pub data: [u8; TCP_CHUNK_BUFFER_SIZE],
    pub to: Channel,
    pub stream_id: Uuid,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum TuiServerMsg {
    Text(TextMsg),
    FileChunk(Chunk),
    FileMetadata(FileMetadata),
    Logout,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ServerTuiMsg {
    Text(TextMsg),
    FileChunk(Chunk),
    FileMetadata(FileMetadata),
    RoomUpdate(RoomChannel),
    JoinRoom(RoomChannel),
    Auth(AuthResponse),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FileMetadata {
    pub name: String,
    pub stream_id: Uuid,
    pub to: Channel,
    pub size: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RoomChannel {
    pub id: Uuid,
    pub name: String,
    pub messages: Vec<ChannelMsg>,
    pub users: Vec<User>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DirectChannel {
    pub user: User,
    pub messages: Vec<ChannelMsg>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ChannelMsg {
    TextMsg(TextMsg),
    JoinNotification(User),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AuthData {
    pub username: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum AuthResponse {
    Success(InitClientData),
    Failure(String),
}
