use std::{collections::HashMap, env, sync::Mutex};

use once_cell::sync::OnceCell;
use uuid::Uuid;

use crate::{
    client_lib::util::types::ActiveStream,
    shared_lib::types::{InitClientData, TextMsg},
};

#[derive(Debug, Default)]
pub struct AppState {
    pub id: Uuid,
    pub active_streams: HashMap<Uuid, ActiveStream>,
    pub direct_messages: Vec<TextMsg>,
    pub room_messages: Vec<TextMsg>,
    pub username: String,
}

static GLOBAL: OnceCell<Mutex<AppState>> = OnceCell::new();

pub fn init_app_state(init: InitClientData) {
    let username = env::args().nth(1).unwrap();

    GLOBAL
        .set(Mutex::new(AppState {
            id: init.id,
            active_streams: HashMap::new(),
            direct_messages: vec![],
            room_messages: vec![],
            username,
        }))
        .expect("App already initialized");
}
//
pub fn get_app_state() -> std::sync::MutexGuard<'static, AppState> {
    GLOBAL
        .get()
        .expect("AEpp state not initialized")
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}
