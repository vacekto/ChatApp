use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::TCP_CHUNK_BUFFER_SIZE;

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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ImgRender {
    pub cache: String,
    pub from: User,
    pub to: Channel,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ClientServerMsg {
    Text(TextMsg),
    ASCII(ImgRender),
    FileChunk(Chunk),
    FileMetadata(FileMetadata),
    Logout,
    CreateRoom(RoomUpdateTransit),
    JoinRoom(RoomUpdateTransit),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RoomUpdateTransit {
    pub room_name: String,
    pub room_password: Option<String>,
}

pub type CreateRoomRes = Result<RoomData, String>;

#[derive(Deserialize, Serialize, Debug)]
pub enum ServerClientMsg {
    Text(TextMsg),
    ASCII(ImgRender),
    FileChunk(Chunk),
    FileMetadata(FileMetadata),
    UserJoinedRoom(JoinRoomNotification),
    UserLeftRoom(LeaveRoomNotification),
    Auth(AuthResponse),
    Register(RegisterResponse),
    Init(UserInitData),
    UserConnected(User),
    UserDisconnected(User),
    CreateRoomResponse(CreateRoomRes),
    JoinRoomResponse(RoomActionRes),
}

pub type RoomActionRes = Result<RoomData, String>;

#[derive(Deserialize, Serialize, Debug)]
pub struct JoinRoomNotification {
    pub user: User,
    pub room_id: Uuid,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LeaveRoomNotification {
    pub user: User,
    pub room_id: Uuid,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TuiRoom {
    pub id: Uuid,
    pub name: String,
    pub messages: VecDeque<ChannelMsg>,
    pub users: Vec<User>,
    pub users_online: Vec<User>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UserInitData {
    pub rooms: Vec<RoomData>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RoomData {
    pub id: Uuid,
    pub name: String,
    pub users: Vec<User>,
    pub users_online: Vec<User>,
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
pub struct DirectChannel {
    pub user: User,
    pub messages: VecDeque<ChannelMsg>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ChannelMsg {
    TextMsg(TextMsg),
    JoinNotification(User),
    Img(ImgRender),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AuthData {
    pub username: String,
    pub pwd: String,
}

pub type AuthResponse = Result<User, String>;

pub type RegisterResponse = Result<User, String>;

#[derive(Deserialize, Serialize, Debug)]
pub enum ClientServerAuthMsg {
    Login(AuthData),
    Register(RegisterData),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RegisterData {
    pub username: String,
    pub pwd: String,
}
