use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::client_lib::util::config::TCP_CHUNK_BUFFER_SIZE;

use super::config::PUBLIC_ROOM_ID_STR;

#[derive(Serialize, Deserialize, Debug)]
pub struct InitClientData {
    pub id: Uuid,
    pub room_channels: Vec<RoomChannel>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub username: String,
    pub id: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Channel {
    Room(Uuid),
    User(Uuid),
}

impl Default for Channel {
    fn default() -> Self {
        Channel::Room(Uuid::from_str(PUBLIC_ROOM_ID_STR).unwrap())
    }
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
pub enum ClientServerMsg {
    // contains username
    InitClient(String),
    Text(TextMsg),
    FileChunk(Chunk),
    FileMetadata(FileMetadata),
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ServerClientMsg {
    InitClient(InitClientData),
    Text(TextMsg),
    FileChunk(Chunk),
    FileMetadata(FileMetadata),
    UserJoinedRoom(RoomJoinNotification),
    JoinInv(RoomChannel),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RoomJoinNotification {
    pub room_id: Uuid,
    pub user: User,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FileMetadata {
    pub name: String,
    pub stream_id: Uuid,
    pub to: Channel,
    pub size: u64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RoomChannel {
    pub id: Uuid,
    pub name: String,
    pub messages: Vec<ChannelMsg>,
    pub users: Vec<User>,
}

#[derive(Deserialize, Serialize)]
pub struct DirectChannel {
    pub user: User,
    pub messages: Vec<ChannelMsg>,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ChannelMsg {
    TextMsg(TextMsg),
    JoinNotification(RoomJoinNotification),
}
