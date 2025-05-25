use crate::client_lib::util::{config::TCP_CHUNK_BUFFER_SIZE, types::ImgRender};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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
pub enum ClientServerTuiMsg {
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
    UserJoinedRoom(RoomUpdateTransit),
    UserLeftRoom(RoomUpdateTransit),
    Auth(AuthResponse),
    Register(RegisterResponse),
    Init(UserClientData),
    UserConnected(User),
    UserDisconnected(User),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RoomUpdateTransit {
    pub user: User,
    pub room_id: Uuid,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TuiRoom {
    pub id: Uuid,
    pub name: String,
    pub messages: VecDeque<TuiMsg>,
    pub users: Vec<User>,
    pub users_online: Vec<User>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UserClientData {
    pub rooms: Vec<TuiRoom>,
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
    pub password: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum AuthResponse {
    Success(User),
    Failure(String),
}

#[derive(Deserialize, Serialize, Debug)]
pub enum RegisterResponse {
    Success(User),
    Failure(String),
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ClientServerConnectMsg {
    Login(AuthData),
    Register(RegisterData),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RegisterData {
    pub username: String,
    pub password: String,
}
