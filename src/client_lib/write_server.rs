use std::io::Write;

use anyhow::{Context, Result};
use crossbeam::select;

use crate::{
    client_lib::global_states::console_logger::console_log, shared_lib::types::ClientServerMsg,
};

use super::global_states::app_state::get_global_state;

pub fn write_to_server() -> Result<()> {
    let state = get_global_state();
    let mut tcp = state.tcp.try_clone().unwrap();
    let rx_file = state.tui_tcp_file_channel.rx.clone();
    let rx_msg = state.tui_tcp_msg_channel.rx.clone();

    drop(state);

    loop {
        if let Ok(msg) = rx_msg.try_recv() {
            let framed: Vec<u8> = frame_tcp_msg(msg)?;
            console_log("msg");
            tcp.write_all(&framed)?;
        }

        select! {
            recv(rx_file) -> chunk => if let Ok(chunk) = chunk {
                let framed = frame_tcp_msg(ClientServerMsg::FileChunk(chunk))?;
                tcp.write_all(&framed)?;
            },
            recv(rx_msg) -> msg => if let Ok(msg) = msg {
                let framed = frame_tcp_msg(msg)?;
                tcp.write_all(&framed)?;
            },
        }
    }
}

fn frame_tcp_msg(msg: ClientServerMsg) -> Result<Vec<u8>> {
    let serialized = bincode::serialize(&msg).context("incorrect init data from server")?;
    let framed = frame_data(&serialized);
    Ok(framed)
}

pub fn frame_data(data: &[u8]) -> Vec<u8> {
    let size = data.len();

    let mut framed: Vec<u8> = vec![
        (size >> 24) as u8,
        (size >> 16) as u8,
        (size >> 8) as u8,
        size as u8,
    ];

    framed.extend_from_slice(&data);

    framed
}
