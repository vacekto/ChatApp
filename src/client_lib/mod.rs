use once_cell::sync::OnceCell;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug)]
pub struct AppState {
    id: Uuid,
}

static GLOBAL: OnceCell<Mutex<AppState>> = OnceCell::new();

pub fn init_global_state(id: Uuid) {
    GLOBAL
        .set(Mutex::new(AppState { id }))
        .expect("Global already initialized");
}

pub async fn get_global_state() -> tokio::sync::MutexGuard<'static, AppState> {
    GLOBAL.get().expect("Global not initialized").lock().await
}

pub struct Global;

impl Global {}
