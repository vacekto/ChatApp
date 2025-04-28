use std::{
    io::{BufReader, ErrorKind, Read},
    net::TcpStream,
    sync::mpsc,
};

use anyhow::{Context, Result};

use crate::shared_lib::types::ServerClientMsg;

use super::util::config::TCP_FRAME_SIZE_HEADER;

pub fn tcp_read(mut tcp: TcpStream, tx_read_tui: mpsc::Sender<ServerClientMsg>) -> Result<()> {
    loop {
        let bytes = read_framed_tcp_msg(&mut tcp)?;
        let msg: ServerClientMsg = bincode::deserialize(&bytes)?;

        tx_read_tui.send(msg)?;
    }
}

pub fn read_framed_tcp_msg(tcp: &mut TcpStream) -> Result<Vec<u8>> {
    let mut size_buf = [0u8; TCP_FRAME_SIZE_HEADER];

    let mut tcp = BufReader::new(tcp);

    match tcp.read_exact(&mut size_buf) {
        Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
            todo!("server dropped... !!.       !");
        }
        Err(e) => return Err(e).context("Error reading TCP size header"),
        _ => {}
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
