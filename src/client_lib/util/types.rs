use crate::shared_lib::types::{DirectChannel, RoomChannel, ServerTuiMsg, TextMsg, User};
use ratatui::crossterm::event::Event;
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
}
pub enum ClientTuiMsg {
    Text(ClientTextMessage),
}

pub enum TuiUpdate {
    ServerMsg(ServerTuiMsg),
    Event(Event),
}

pub struct ClientTextMessage {
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
