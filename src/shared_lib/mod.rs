use std::env;

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

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct TextMessage(pub String);

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct TextMetadata {
    pub sender: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct FileMetadata {
    pub sender: String,
    pub size: u32,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct MsgMetadata {
    // Text(TextMetadata),
    // File(FileMetadata),
    pub sender: String,
    pub size: usize,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct InitClientData {
    pub id: Uuid,
}
