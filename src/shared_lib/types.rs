use crate::client_lib::util::{config::TCP_CHUNK_BUFFER_SIZE, types::ImgRender};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InitUserData {
    pub id: Uuid,
    pub username: String,
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
pub enum ClientServerMsg {
    Text(TextMsg),
    FileChunk(Chunk),
    FileMetadata(FileMetadata),
    Logout,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ServerClientMsg {
    Text(TextMsg),
    FileChunk(Chunk),
    FileMetadata(FileMetadata),
    UserJoinedRoom(ClientRoomUpdateTransit),
    UserLeftRoom(ClientRoomUpdateTransit),
    Auth(AuthResponse),
    Init(InitPersistedUserData),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ClientRoomUpdateTransit {
    pub user: User,
    pub room_id: Uuid,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct InitPersistedUserData {
    pub rooms: Vec<RoomChannel>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FileMetadata {
    pub filename: String,
    pub stream_id: Uuid,
    pub to: Channel,
    pub from: Channel,
    pub size: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RoomChannel {
    pub id: Uuid,
    pub name: String,
    pub messages: VecDeque<TuiMsg>,
    pub users: Vec<User>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DirectChannel {
    pub user: User,
    pub messages: VecDeque<TuiMsg>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum TuiMsg {
    TextMsg(TextMsg),
    JoinNotification(User),
    Img(ImgRender),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AuthData {
    pub username: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum AuthResponse {
    Success(InitUserData),
    Failure(String),
}
