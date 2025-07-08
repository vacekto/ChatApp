use crate::{
    server_lib::util::types::server_data_types::WssClientWrite,
    shared_lib::types::{Chunk, ClientServerConnectMsg, ClientServerMsg},
};
use anyhow::Result;
use futures::SinkExt;
use tokio::select;

pub async fn write_to_server(
    mut wss: WssClientWrite,
    mut rx_tui_tcp_msg: tokio::sync::mpsc::Receiver<ClientServerMsg>,
    mut rx_tui_tcp_file: tokio::sync::mpsc::Receiver<Chunk>,
    mut rx_tui_tcp_auth: tokio::sync::mpsc::Receiver<ClientServerConnectMsg>,
) -> Result<()> {
    loop {
        if let Ok(msg) = rx_tui_tcp_msg.try_recv() {
            let serialized = bincode::serialize(&msg)?;
            // let framed = frame_data(&serialized);
            // let ws_msg= Message::Binary(framed)
            wss.send(serialized.into()).await?;
        }

        select! {
            result = rx_tui_tcp_file.recv() => match result {
                Some(chunk) => {
                    let serialized = bincode::serialize(&ClientServerMsg::FileChunk(chunk))?;
                    // let framed = frame_data(&serialized);
                    wss.send(serialized.into()).await?;
                },
                _ => {}
            },


            result = rx_tui_tcp_msg.recv() => match result {
                Some(msg) => {
                    let serialized = bincode::serialize(&msg)?;
                    // let framed = frame_data(&serialized);
                    wss.send(serialized.into()).await?;
                },
                _ => {}
            },

            result = rx_tui_tcp_auth.recv() => if let Some(msg) = result {
                let serialized = bincode::serialize(&msg)?;
                // let framed = frame_data(&serialized);
                wss.send(serialized.into()).await?;

            }
        }
    }
}

// pub fn frame_data(data: &[u8]) -> Vec<u8> {
//     let size = data.len();

//     let mut framed: Vec<u8> = vec![
//         (size >> 24) as u8,
//         (size >> 16) as u8,
//         (size >> 8) as u8,
//         size as u8,
//     ];

//     framed.extend_from_slice(&data);
//     framed
// }
