use super::{global_states::app_state::get_global_state, util::config::TCP_FRAME_SIZE_HEADER};
use crate::shared_lib::types::ServerTuiMsg;
use anyhow::{Context, Result};
use std::{
    io::{BufReader, ErrorKind, Read},
    net::TcpStream,
};

pub fn tcp_read() -> Result<()> {
    let state = get_global_state();
    let mut tcp = state.tcp.try_clone().unwrap();
    let tx_tcp_tui = state.tcp_tui_channel.tx.clone();
    drop(state);

    loop {
        let bytes = read_framed_tcp_msg(&mut tcp)?;
        let msg: ServerTuiMsg = bincode::deserialize(&bytes)?;
        tx_tcp_tui.send(msg)?;
    }
}

pub fn read_framed_tcp_msg(tcp: &mut TcpStream) -> Result<Vec<u8>> {
    let mut size_buf = [0u8; TCP_FRAME_SIZE_HEADER];

    let mut tcp = BufReader::new(tcp);

    match tcp.read_exact(&mut size_buf) {
        Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
            todo!("server dropped");
        }
        Err(e) => Err(e)?,
        Ok(_) => {}
    }

    let size = ((size_buf[0] as usize) << 24)
        + ((size_buf[1] as usize) << 16)
        + ((size_buf[2] as usize) << 8)
        + size_buf[3] as usize;

    let mut data = vec![0u8; size];

    tcp.read_exact(&mut data)
        .context("closed connection while reading data of framed message")
        .unwrap();

    Ok(data)
}
