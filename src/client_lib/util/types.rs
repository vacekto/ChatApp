use crate::shared_lib::types::{
    AuthResponse, Channel, Chunk, DirectChannel, FileMetadata, JoinRoomNotification,
    LeaveRoomNotification, RegisterResponse, Response, RoomData, TextMsg, TuiRoom, User,
    UserInitData,
};
use ratatui::crossterm::event::Event;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, sync::mpsc};
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
    CrosstermEvent(Event),
    Img(ImgRender),
    Text(TextMsg),
    UserJoinedRoom(JoinRoomNotification),
    UserLeftRoom(LeaveRoomNotification),
    JoinRoom(Response<RoomData>),
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ImgRender {
    pub cache: String,
    pub from: User,
    pub to: Channel,
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
    Login,
    Register,
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
pub struct CrossbemChannel<T, R> {
    pub tx: crossbeam::channel::Sender<T>,
    pub rx: crossbeam::channel::Receiver<R>,
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
pub enum TcpStreamMsg {
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
