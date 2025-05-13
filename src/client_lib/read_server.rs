use super::{
    global_states::app_state::get_global_state,
    util::{
        config::TCP_FRAME_SIZE_HEADER,
        types::{TcpStreamMsg, TuiUpdate},
    },
};
use crate::shared_lib::types::ServerClientMsg;
use anyhow::{Context, Result};
use std::{
    io::{BufReader, ErrorKind, Read},
    net::TcpStream,
};

pub fn listen_for_server() -> Result<()> {
    let state = get_global_state();
    let mut tcp = state.tcp.try_clone().unwrap();
    let tx_tui_update = state.tui_update_channel.tx.clone();
    let tx_tcp_stream = state.tcp_stream_channel.tx.clone();

    drop(state);

    let mut tcp: BufReader<&mut TcpStream> = BufReader::new(&mut tcp);

    loop {
        let bytes = read_framed_tcp_msg(&mut tcp)?;
        let msg: ServerClientMsg = bincode::deserialize(&bytes)?;

        match msg {
            ServerClientMsg::FileMetadata(data) => {
                tx_tcp_stream.send(TcpStreamMsg::FileMetadata(data))?
            }
            ServerClientMsg::FileChunk(chunk) => {
                tx_tcp_stream.send(TcpStreamMsg::FileChunk(chunk))?
            }
            ServerClientMsg::Auth(auth) => tx_tui_update.send(TuiUpdate::Auth(auth))?,
            ServerClientMsg::JoinRoom(room) => tx_tui_update.send(TuiUpdate::JoinRoom(room))?,
            ServerClientMsg::RoomUpdate(update) => {
                tx_tui_update.send(TuiUpdate::RoomUpdate(update))?
            }
            ServerClientMsg::Text(msg) => tx_tui_update.send(TuiUpdate::Text(msg))?,
        }
    }
}

pub fn read_framed_tcp_msg(tcp: &mut BufReader<&mut TcpStream>) -> Result<Vec<u8>> {
    let mut size_buf = [0u8; TCP_FRAME_SIZE_HEADER];

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
