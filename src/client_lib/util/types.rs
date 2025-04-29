use std::{collections::HashMap, fs::File};

use ratatui::crossterm::event::Event;
use uuid::Uuid;

use crate::shared_lib::types::{DirectChannel, RoomChannel, ServerTuiMsg, TextMsg, User};

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
