use std::{io::Write, net::TcpStream, sync::mpsc};

use anyhow::{Context, Result};

use crate::shared_lib::types::TuiServerMsg;

pub fn tcp_write(mut tcp: TcpStream, rx: mpsc::Receiver<TuiServerMsg>) -> Result<()> {
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
