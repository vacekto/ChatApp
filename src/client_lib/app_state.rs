use std::collections::HashMap;

use once_cell::sync::OnceCell;
use tokio::sync::Mutex;
use uuid::Uuid;

use super::types::ActiveStream;

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

pub async fn get_global_state() -> tokio::sync::MutexGuard<'static, AppState> {
    GLOBAL
        .get()
        .expect("Global state not initialized")
        .lock()
        .await
}
