use crate::{
    client_lib::util::types::MpscChannel,
    shared_lib::types::{ServerTuiMsg, TuiServerMsg},
};
use once_cell::sync::OnceCell;
use std::{net::TcpStream, sync::Mutex};

#[derive(Debug)]
pub struct GlobalData {
    pub tcp: TcpStream,
    pub tcp_tui_channel: MpscChannel<ServerTuiMsg, ServerTuiMsg>,
    pub tui_tcp_channel: MpscChannel<TuiServerMsg, TuiServerMsg>,
}

static GLOBAL: OnceCell<Mutex<GlobalData>> = OnceCell::new();

pub fn init_global_state(
    tcp: TcpStream,
    tcp_tui_channel: MpscChannel<ServerTuiMsg, ServerTuiMsg>,
    tui_tcp_channel: MpscChannel<TuiServerMsg, TuiServerMsg>,
) {
    GLOBAL
        .set(Mutex::new(GlobalData {
            tcp,
            tcp_tui_channel,
            tui_tcp_channel,
        }))
        .expect("Global state already initialized");
}
//
pub fn get_global_state() -> std::sync::MutexGuard<'static, GlobalData> {
    GLOBAL
        .get()
        .expect("App state not initialized")
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}
