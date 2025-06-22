use crate::shared_lib::types::{Chunk, ClientServerConnectMsg, ClientServerMsg};
use anyhow::Result;
use tokio::{
    io::{AsyncWriteExt, WriteHalf},
    net::TcpStream,
    select,
};
use tokio_rustls::client::TlsStream;

pub async fn write_to_server(
    mut tcp: WriteHalf<TlsStream<TcpStream>>,
    mut rx_tui_tcp_msg: tokio::sync::mpsc::Receiver<ClientServerMsg>,
    mut rx_tui_tcp_file: tokio::sync::mpsc::Receiver<Chunk>,
    mut rx_tui_tcp_auth: tokio::sync::mpsc::Receiver<ClientServerConnectMsg>,
) -> Result<()> {
    loop {
        if let Ok(msg) = rx_tui_tcp_msg.try_recv() {
            let serialized = bincode::serialize(&msg)?;
            let framed = frame_data(&serialized);
            tcp.write_all(&framed).await?;
        }

        select! {
            result = rx_tui_tcp_file.recv() => match result {
                Some(chunk) => {
                    let serialized = bincode::serialize(&ClientServerMsg::FileChunk(chunk))?;
                    let framed = frame_data(&serialized);
                    tcp.write_all(&framed).await?;
                },
                _ => {}
            },


            result = rx_tui_tcp_msg.recv() => match result {
                Some(msg) => {
                    let serialized = bincode::serialize(&msg)?;
                    let framed = frame_data(&serialized);
                    tcp.write_all(&framed).await?;
                },
                _ => {}
            },

            result = rx_tui_tcp_auth.recv() => if let Some(msg) = result {
                let serialized = bincode::serialize(&msg)?;
                let framed = frame_data(&serialized);
                tcp.write_all(&framed).await?;

            }
        }
    }
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
