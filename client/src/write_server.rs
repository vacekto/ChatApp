use anyhow::Result;
use futures::SinkExt;
use shared::types::{Chunk, ClientServerConnectMsg, ClientServerMsg};
use tokio::select;

use crate::util::types::WsWrite;

pub async fn write_to_server(
    mut ws: WsWrite,
    mut rx_tui_ws_msg: tokio::sync::mpsc::Receiver<ClientServerMsg>,
    mut rx_tui_ws_file: tokio::sync::mpsc::Receiver<Chunk>,
    mut rx_tui_ws_auth: tokio::sync::mpsc::Receiver<ClientServerConnectMsg>,
) -> Result<()> {
    loop {
        if let Ok(msg) = rx_tui_ws_msg.try_recv() {
            let serialized = bincode::serialize(&msg)?;
            // let framed = frame_data(&serialized);
            // let ws_msg= Message::Binary(framed)
            ws.send(serialized.into()).await?;
        }

        select! {
            result = rx_tui_ws_file.recv() => match result {
                Some(chunk) => {
                    let serialized = bincode::serialize(&ClientServerMsg::FileChunk(chunk))?;
                    // let framed = frame_data(&serialized);
                    ws.send(serialized.into()).await?;
                },
                _ => {}
            },


            result = rx_tui_ws_msg.recv() => match result {
                Some(msg) => {
                    let serialized = bincode::serialize(&msg)?;
                    // let framed = frame_data(&serialized);
                    ws.send(serialized.into()).await?;
                },
                _ => {}
            },

            result = rx_tui_ws_auth.recv() => if let Some(msg) = result {
                let serialized = bincode::serialize(&msg)?;
                // let framed = frame_data(&serialized);
                ws.send(serialized.into()).await?;

            }
        }
    }
}
