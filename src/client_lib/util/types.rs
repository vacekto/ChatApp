use std::{collections::HashMap, fs::File};

use ratatui::crossterm::event::Event;
use uuid::Uuid;

use crate::shared_lib::types::{ServerClientMsg, TextMsg, User};

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
    ServerMsg(ServerClientMsg),
    Event(Event),
}

pub struct ClientTextMessage {
    pub text: String,
    pub from: User,
}
