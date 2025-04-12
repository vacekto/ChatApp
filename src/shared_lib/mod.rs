use std::env;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

pub fn get_addr(default_hostname: &str, default_port: &str) -> String {
    let mut args = env::args();

    let hostname = match args.nth(1) {
        Some(h) => h,
        None => String::from(default_hostname),
    };

    let port = match args.nth(2) {
        Some(p) => p,
        None => String::from(default_port),
    };

    hostname + ":" + &port
}

#[derive(Serialize, Deserialize)]
pub struct InitClientData {
    pub id: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct User {
    // username: String,
    pub id: Uuid,
}

#[derive(Serialize, Deserialize)]
pub enum Channel {
    Room(Uuid),
    Direct(Uuid),
}

pub struct DirectChannel {
    pub tx: mpsc::Sender<Bytes>,
    pub rx: mpsc::Receiver<Bytes>,
}

pub struct RoomChannel {
    pub tx: broadcast::Sender<Bytes>,
    pub rx: broadcast::Receiver<Bytes>,
}

#[derive(Serialize, Deserialize)]
pub struct TextMessage {
    pub text: String,
    pub from: User,
    pub to: Channel,
}
#[derive(Serialize, Deserialize)]
pub struct Chunk {
    from: User,
    #[serde(with = "serde_bytes")]
    data: [u8; 8192],
    to: Uuid,
    is_last: bool,
}

#[derive(Deserialize, Serialize)]
pub enum MessageToServer {
    Text(TextMessage),
    File(Chunk),
}
