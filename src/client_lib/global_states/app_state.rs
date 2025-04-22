use std::{collections::HashMap, sync::Mutex};

use once_cell::sync::OnceCell;
use uuid::Uuid;

use crate::client_lib::util::types::ActiveStream;

#[derive(Debug)]
pub struct AppState {
    pub id: Uuid,
    pub active_streams: HashMap<Uuid, ActiveStream>,
}

static GLOBAL: OnceCell<Mutex<AppState>> = OnceCell::new();

pub fn init_app_state(id: Uuid) {
    GLOBAL
        .set(Mutex::new(AppState {
            id,
            active_streams: HashMap::new(),
        }))
        .expect("App already initialized");
}

pub fn get_app_state() -> std::sync::MutexGuard<'static, AppState> {
    GLOBAL
        .get()
        .expect("App state not initialized")
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}
