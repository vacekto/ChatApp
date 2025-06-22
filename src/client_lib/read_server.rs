use super::util::{
    config::TCP_FRAME_SIZE_HEADER,
    types::{TcpStreamMsg, TuiUpdate},
};
use crate::shared_lib::types::ServerClientMsg;
use anyhow::Result;
use tokio::{
    io::{AsyncReadExt, BufReader, ReadHalf},
    net::TcpStream,
};
use tokio_rustls::client::TlsStream;

pub async fn listen_for_server(
    mut tcp: ReadHalf<TlsStream<TcpStream>>,
    tx_tcp_tui: tokio::sync::mpsc::Sender<TuiUpdate>,
    tx_tcp_stream: tokio::sync::mpsc::Sender<TcpStreamMsg>,
) -> Result<()> {
    let mut tcp: BufReader<&mut ReadHalf<TlsStream<TcpStream>>> = BufReader::new(&mut tcp);
    loop {
        let bytes = read_framed_tcp_msg(&mut tcp).await?;
        let msg: ServerClientMsg = bincode::deserialize(&bytes)?;

        match msg {
            ServerClientMsg::FileMetadata(data) => {
                tx_tcp_stream.send(TcpStreamMsg::FileMetadata(data)).await?
            }
            ServerClientMsg::FileChunk(chunk) => {
                tx_tcp_stream.send(TcpStreamMsg::FileChunk(chunk)).await?
            }
            ServerClientMsg::UserJoinedRoom(update) => {
                tx_tcp_tui.send(TuiUpdate::UserJoinedRoom(update)).await?
            }
            ServerClientMsg::Text(msg) => tx_tcp_tui.send(TuiUpdate::Text(msg)).await?,
            ServerClientMsg::Init(data) => tx_tcp_tui.send(TuiUpdate::Init(data)).await?,
            ServerClientMsg::UserLeftRoom(update) => {
                tx_tcp_tui.send(TuiUpdate::UserLeftRoom(update)).await?
            }
            ServerClientMsg::Auth(auth) => tx_tcp_tui.send(TuiUpdate::Auth(auth)).await?,
            ServerClientMsg::Register(res) => {
                tx_tcp_tui.send(TuiUpdate::RegisterResponse(res)).await?
            }
            ServerClientMsg::UserConnected(user) => {
                tx_tcp_tui.send(TuiUpdate::UserConnected(user)).await?
            }
            ServerClientMsg::UserDisconnected(user) => {
                tx_tcp_tui.send(TuiUpdate::UserDisconnected(user)).await?
            }
            ServerClientMsg::CreateRoomResponse(res) => {
                tx_tcp_tui.send(TuiUpdate::JoinRoom(res)).await?
            }
            ServerClientMsg::JoinRoomResponse(res) => {
                tx_tcp_tui.send(TuiUpdate::JoinRoom(res)).await?
            }
            ServerClientMsg::ASCII(img) => tx_tcp_tui.send(TuiUpdate::Img(img)).await?,
        }
    }
}

async fn read_framed_tcp_msg(
    tcp: &mut BufReader<&mut ReadHalf<TlsStream<TcpStream>>>,
) -> Result<Vec<u8>> {
    let mut size_buf = [0u8; TCP_FRAME_SIZE_HEADER];

    tcp.read_exact(&mut size_buf).await?;

    let size = ((size_buf[0] as usize) << 24)
        + ((size_buf[1] as usize) << 16)
        + ((size_buf[2] as usize) << 8)
        + size_buf[3] as usize;

    let mut data = vec![0u8; size];
    tcp.read_exact(&mut data).await?;

    Ok(data)
}
