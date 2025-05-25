use super::{
    global_states::app_state::get_global_state,
    util::{
        config::TCP_FRAME_SIZE_HEADER,
        types::{TcpStreamMsg, TuiUpdate},
    },
};
use crate::shared_lib::types::ServerClientMsg;
use anyhow::Result;
use std::{
    io::{BufReader, Read},
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
            ServerClientMsg::UserJoinedRoom(update) => {
                tx_tui_update.send(TuiUpdate::UserJoinedRoom(update))?
            }
            ServerClientMsg::Text(msg) => tx_tui_update.send(TuiUpdate::Text(msg))?,
            ServerClientMsg::Init(data) => tx_tui_update.send(TuiUpdate::User(data))?,
            ServerClientMsg::UserLeftRoom(update) => {
                tx_tui_update.send(TuiUpdate::UserLeftRoom(update))?
            }
            ServerClientMsg::Auth(auth) => tx_tui_update.send(TuiUpdate::Auth(auth))?,
            ServerClientMsg::Register(res) => {
                tx_tui_update.send(TuiUpdate::RegisterResponse(res))?
            }
            ServerClientMsg::UserConnected(user) => {
                tx_tui_update.send(TuiUpdate::UserConnected(user))?
            }
            ServerClientMsg::UserDisconnected(user) => {
                tx_tui_update.send(TuiUpdate::UserDisconnected(user))?
            }
        }
    }
}

pub fn read_framed_tcp_msg(tcp: &mut BufReader<&mut TcpStream>) -> Result<Vec<u8>> {
    let mut size_buf = [0u8; TCP_FRAME_SIZE_HEADER];

    tcp.read_exact(&mut size_buf)?;

    let size = ((size_buf[0] as usize) << 24)
        + ((size_buf[1] as usize) << 16)
        + ((size_buf[2] as usize) << 8)
        + size_buf[3] as usize;

    let mut data = vec![0u8; size];
    tcp.read_exact(&mut data)?;

    Ok(data)
}
