use crate::util::types::{CrossbemChannel, MpscChannel, TcpStreamMsg, TuiUpdate};
use once_cell::sync::OnceCell;
use shared::types::{Chunk, ClientServerMsg};
use std::{
    net::TcpStream,
    sync::{Mutex, mpsc},
};

#[derive(Debug)]
pub struct GlobalData {
    pub tcp: TcpStream,
    pub tui_tcp_file_channel: CrossbemChannel<Chunk, Chunk>,
    pub tui_tcp_msg_channel: CrossbemChannel<ClientServerMsg, ClientServerMsg>,
    pub tcp_stream_channel: MpscChannel<TcpStreamMsg, TcpStreamMsg>,
    pub tui_update_channel: MpscChannel<TuiUpdate, TuiUpdate>,
}

static GLOBAL: OnceCell<Mutex<GlobalData>> = OnceCell::new();

pub fn init_global_state(tcp: TcpStream) {
    let (tx_tui_tcp_msg, rx_tui_tcp_msg) = crossbeam::channel::bounded::<ClientServerMsg>(30);
    let (tx_tui_tcp_file, rx_tui_tcp_file) = crossbeam::channel::bounded::<Chunk>(1000);
    let (tx_tcp_stream, rx_tcp_stream) = mpsc::channel::<TcpStreamMsg>();
    let (tx_tui_update, rx_tui_update) = mpsc::channel::<TuiUpdate>();

    let tcp_stream_channel = MpscChannel {
        tx: tx_tcp_stream,
        rx: Some(rx_tcp_stream),
    };

    let tui_update_channel = MpscChannel {
        tx: tx_tui_update,
        rx: Some(rx_tui_update),
    };

    let tui_tcp_file_channel = CrossbemChannel {
        tx: tx_tui_tcp_file,
        rx: rx_tui_tcp_file,
    };

    let tui_tcp_msg_channel = CrossbemChannel {
        tx: tx_tui_tcp_msg,
        rx: rx_tui_tcp_msg,
    };

    GLOBAL
        .set(Mutex::new(GlobalData {
            tcp,
            tui_tcp_file_channel,
            tui_tcp_msg_channel,
            tcp_stream_channel,
            tui_update_channel,
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
