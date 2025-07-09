use crate::util::types::{TuiUpdate, WsRead, WsStreamMsg};
use anyhow::Result;
use futures::StreamExt;
use shared::types::ServerClientMsg;
use tokio_tungstenite::tungstenite::Message;

pub async fn listen_for_server(
    mut ws: WsRead,
    tx_wss_tui: tokio::sync::mpsc::Sender<TuiUpdate>,
    tx_wss_stream: tokio::sync::mpsc::Sender<WsStreamMsg>,
) -> Result<()> {
    loop {
        if let Some(Ok(ws_msg)) = ws.next().await {
            let server_msg: ServerClientMsg = match ws_msg {
                Message::Binary(data) => bincode::deserialize(&data)?,
                _ => unreachable!("unimplemented handler for web socket message"),
            };
            match server_msg {
                ServerClientMsg::FileMetadata(data) => {
                    tx_wss_stream.send(WsStreamMsg::FileMetadata(data)).await?
                }
                ServerClientMsg::FileChunk(chunk) => {
                    tx_wss_stream.send(WsStreamMsg::FileChunk(chunk)).await?
                }
                ServerClientMsg::UserJoinedRoom(update) => {
                    tx_wss_tui.send(TuiUpdate::UserJoinedRoom(update)).await?
                }
                ServerClientMsg::Text(msg) => tx_wss_tui.send(TuiUpdate::Text(msg)).await?,
                ServerClientMsg::Init(data) => tx_wss_tui.send(TuiUpdate::Init(data)).await?,
                ServerClientMsg::UserLeftRoom(update) => {
                    tx_wss_tui.send(TuiUpdate::UserLeftRoom(update)).await?
                }
                ServerClientMsg::Auth(auth) => tx_wss_tui.send(TuiUpdate::Auth(auth)).await?,
                ServerClientMsg::Register(res) => {
                    tx_wss_tui.send(TuiUpdate::RegisterResponse(res)).await?
                }
                ServerClientMsg::UserConnected(user) => {
                    tx_wss_tui.send(TuiUpdate::UserConnected(user)).await?
                }
                ServerClientMsg::UserDisconnected(user) => {
                    tx_wss_tui.send(TuiUpdate::UserDisconnected(user)).await?
                }
                ServerClientMsg::CreateRoomResponse(res) => {
                    tx_wss_tui.send(TuiUpdate::JoinRoom(res)).await?
                }
                ServerClientMsg::JoinRoomResponse(res) => {
                    tx_wss_tui.send(TuiUpdate::JoinRoom(res)).await?
                }
                ServerClientMsg::ASCII(img) => tx_wss_tui.send(TuiUpdate::Img(img)).await?,
            };
        }
    }
}
