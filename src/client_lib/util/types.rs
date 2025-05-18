use crate::shared_lib::types::{
    AuthResponse, Channel, Chunk, ClientRoomUpdateTransit, DirectChannel, FileMetadata,
    InitPersistedUserData, RoomChannel, TextMsg, User,
};
use ratatui::crossterm::event::Event;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, path::PathBuf, sync::mpsc};
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
    UserJoinedRoom(ClientRoomUpdateTransit),
    UserLeftRoom(ClientRoomUpdateTransit),
    Auth(AuthResponse),
    UserInitData(InitPersistedUserData),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ImgRender {
    pub cache: String,
    pub from: Channel,
}

pub struct TuiTextMessage {
    pub text: String,
    pub from: User,
}

pub enum Contact<'a> {
    Direct(&'a DirectChannel),
    Room(&'a RoomChannel),
}

pub enum ChannelKind {
    Room,
    Direct,
}

pub struct ActiveChannel {
    pub kind: ChannelKind,
    pub id: Option<Uuid>,
}

pub enum ActiveScreen {
    Main,
    Login,
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

pub struct FileSelector {
    pub current_location: PathBuf,
    pub selected_index: usize,
    pub entries: Vec<SelectorEntry>,
    pub scroll_offset: u16,
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
