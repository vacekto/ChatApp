use std::{collections::HashMap, sync::Mutex};

use once_cell::sync::OnceCell;
use uuid::Uuid;

use super::util::types::ActiveStream;

#[derive(Debug)]
pub struct AppState {
    pub id: Uuid,
    pub active_streams: HashMap<Uuid, ActiveStream>,
}

static GLOBAL: OnceCell<Mutex<AppState>> = OnceCell::new();

pub fn init_global_state(id: Uuid) {
    GLOBAL
        .set(Mutex::new(AppState {
            id,
            active_streams: HashMap::new(),
        }))
        .expect("Global already initialized");
}

pub fn get_global_state() -> std::sync::MutexGuard<'static, AppState> {
    GLOBAL
        .get()
        .expect("Global state not initialized")
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}
