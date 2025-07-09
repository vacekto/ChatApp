use futures::stream::{SplitSink, SplitStream};
use shared::types::{
    AuthResponse, Channel, Chunk, DirectChannel, FileMetadata, ImgRender, JoinRoomNotification,
    LeaveRoomNotification, RegisterResponse, RoomData, TextMsg, TuiRoom, User, UserInitData,
};
use std::{collections::HashMap, fs::File, sync::mpsc};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct AppState {
    pub id: Uuid,
    pub username: String,
    pub active_streams: HashMap<Uuid, ActiveStream>,
    pub direct_messages: HashMap<Uuid, Vec<TextMsg>>,
    pub room_messages: HashMap<Uuid, Vec<TextMsg>>,
}

#[derive(Debug)]
pub struct ActiveStream {
    pub file_handle: File,
    pub written: u64,
    pub size: u64,
    pub file_name: String,
    pub from: Channel,
}

#[derive(Debug)]
pub enum TuiUpdate {
    Img(ImgRender),
    Text(TextMsg),
    UserJoinedRoom(JoinRoomNotification),
    UserLeftRoom(LeaveRoomNotification),
    JoinRoom(Result<RoomData, String>),
    Auth(AuthResponse),
    Init(UserInitData),
    UserDisconnected(User),
    UserConnected(User),
    RegisterResponse(RegisterResponse),
}

pub enum Notification {
    Success(String),
    Failure(String),
}

pub struct TuiTextMessage {
    pub text: String,
    pub from: User,
}

pub enum Contact<'a> {
    Direct(&'a DirectChannel),
    Room(&'a TuiRoom),
}

pub enum ChannelKind {
    Room,
    Direct,
}

pub struct ActiveChannel {
    pub kind: ChannelKind,
    pub id: Option<Uuid>,
}

#[derive(PartialEq)]
pub enum ActiveScreen {
    Main,
    Entry,
}

#[derive(Debug, PartialEq)]
pub enum ActiveEntryInput {
    Username,
    Password,
    RepeatPassword,
}

#[derive(PartialEq)]
pub enum ActiveEntryScreen {
    ASLogin,
    ASRegister,
}

#[derive(Clone)]
pub enum AppMsg {
    Quit,
}

#[derive(Debug)]
pub struct MpscChannel<T, R> {
    pub tx: mpsc::Sender<T>,
    pub rx: Option<mpsc::Receiver<R>>,
}

#[derive(Debug)]
pub struct SelectorEntry {
    pub name: String,
    pub kind: SelectorEntryKind,
    pub selected: bool,
}

#[derive(Debug, PartialEq)]
pub enum SelectorEntryKind {
    Folder,
    File,
}

#[derive(Debug)]
pub enum WsStreamMsg {
    FileChunk(Chunk),
    FileMetadata(FileMetadata),
}

#[derive(Debug)]
pub enum Focus {
    Contacts,
    Messages,
}

#[derive(PartialEq, Debug)]
pub enum ActiveCreateRoomInput {
    Name,
    Password,
}

pub enum RoomAction {
    Create,
    Join,
}

#[derive(PartialEq)]
pub enum FileAction {
    ASCII,
    File,
}

pub type WsRead = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;
pub type WsWrite = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
