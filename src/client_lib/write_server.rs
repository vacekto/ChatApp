use std::io::Write;

use anyhow::{Context, Result};

use crate::shared_lib::types::TuiServerMsg;

use super::global_states::app_state::get_global_state;

pub fn tcp_write() -> Result<()> {
    let mut state = get_global_state();
    let mut tcp = state.tcp.try_clone().unwrap();
    let rx = state
        .tui_tcp_channel
        .rx
        .take()
        .expect("rx_tui_tcp already taken, can be listening only once!!");
    drop(state);

    while let Ok(msg) = rx.recv() {
        let framed = frame_tcp_msg(msg)?;
        tcp.write_all(&framed)?;
    }

    Ok(())
}

fn frame_tcp_msg(msg: TuiServerMsg) -> Result<Vec<u8>> {
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
