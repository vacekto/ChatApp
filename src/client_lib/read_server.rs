use super::util::types::{TcpStreamMsg, TuiUpdate};
use crate::{
    server_lib::util::types::server_data_types::WssClientRead, shared_lib::types::ServerClientMsg,
};
use anyhow::Result;
use futures::StreamExt;
use tokio_tungstenite::tungstenite::Message;

pub async fn listen_for_server(
    mut wss: WssClientRead,
    tx_wss_tui: tokio::sync::mpsc::Sender<TuiUpdate>,
    tx_wss_stream: tokio::sync::mpsc::Sender<TcpStreamMsg>,
) -> Result<()> {
    // let mut tcp: BufReader<&mut ReadHalf<TlsStream<TcpStream>>> = BufReader::new(&mut tcp);
    loop {
        // let bytes = read_framed_tcp_msg(&mut wss).await?;
        // let msg: ServerClientMsg = bincode::deserialize(&bytes)?;

        if let Some(Ok(ws_msg)) = wss.next().await {
            let server_msg: ServerClientMsg = match ws_msg {
                Message::Binary(data) => bincode::deserialize(&data)?,
                _ => unreachable!("unimplemented handler for web socket message"),
            };
            match server_msg {
                ServerClientMsg::FileMetadata(data) => {
                    tx_wss_stream.send(TcpStreamMsg::FileMetadata(data)).await?
                }
                ServerClientMsg::FileChunk(chunk) => {
                    tx_wss_stream.send(TcpStreamMsg::FileChunk(chunk)).await?
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

// async fn read_framed_tcp_msg(wss: &mut WssClientRead) -> Result<Vec<u8>> {
//     let mut size_buf = [0u8; TCP_FRAME_SIZE_HEADER];

//     wss.read(&mut size_buf).await?;

//     let size = ((size_buf[0] as usize) << 24)
//         + ((size_buf[1] as usize) << 16)
//         + ((size_buf[2] as usize) << 8)
//         + size_buf[3] as usize;

//     let mut data = vec![0u8; size];
//     wss.read_exact(&mut data).await?;

//     Ok(data)
// }
