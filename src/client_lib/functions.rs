use futures::{SinkExt, StreamExt};
use std::str::FromStr;
use std::{os::unix::fs::MetadataExt, path::Path};
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;

use anyhow::{bail, Context, Result};
use bytes::Bytes;
use tokio::{
    fs::File,
    net::{tcp::OwnedWriteHalf, TcpStream},
};
use tokio_util::codec::{Framed, FramedWrite, LengthDelimitedCodec};
use uuid::Uuid;

use crate::shared_lib::{
    config::PUBLIC_ROOM_ID_STR,
    types::{Channel, Chunk, FileMetadata, InitClientData, ServerMessage, TextMessage, User},
};

use super::config::FILES_DIR;
use super::{
    app_state::{get_global_state, init_global_state},
    types::ActiveStream,
};

pub async fn send_file(tx_parse_write: mpsc::Sender<Bytes>, path: &str) -> Result<()> {
    let path = Path::new(path);
    let stream_id = Uuid::new_v4();
    let mut file = File::open(path).await?;
    let mut buffer = [0u8; 8192];
    let state = get_global_state().await;
    let meta = file.metadata().await?;

    let meta = FileMetadata {
        name: String::from(path.file_name().unwrap().to_str().unwrap()),
        stream_id,
        to: Channel::Room(Uuid::from_str(PUBLIC_ROOM_ID_STR).unwrap()),
        size: meta.size(),
    };

    let server_message = ServerMessage::FileMetadata(meta);

    let server_message = Bytes::from(bincode::serialize(&server_message)?);

    tx_parse_write.send(server_message).await?;

    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        let chunk = Chunk {
            data: buffer.clone(),
            from: User { id: state.id },
            to: Channel::Room(Uuid::from_str(PUBLIC_ROOM_ID_STR).unwrap()),
            stream_id,
        };

        let msg = ServerMessage::FileChunk(chunk);

        let serialized = Bytes::from(
            bincode::serialize(&msg).context("bincode failed to serialize file chunk")?,
        );

        tx_parse_write.send(serialized).await?;
    }
    Ok(())
}

pub async fn send_text_msg(
    tx_parse_write: mpsc::Sender<Bytes>,
    text: &mut String,
    to: Channel,
) -> Result<()> {
    let state = get_global_state().await;
    // let public_room_id = Uuid::from_str(PUBLIC_ROOM_ID_STR).unwrap();

    let text_msg = TextMessage {
        text: text.clone(),
        from: User { id: state.id },
        to,
    };

    let server_msg = ServerMessage::Text(text_msg);
    let serialized_data =
        Bytes::from(bincode::serialize(&server_msg).context("failed bincode writing to server")?);

    tx_parse_write.send(serialized_data).await?;

    Ok(())
}

pub async fn handle_file_metadata(meta: FileMetadata) -> Result<()> {
    let path = String::from(FILES_DIR) + &meta.name;
    let path = Path::new(&path);

    // Create parent directories
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let file: File = File::create(path).await?;
    let mut state = get_global_state().await;
    let stream = ActiveStream {
        file_handle: file,
        written: 0,
        size: meta.size,
    };

    state.active_streams.insert(meta.stream_id, stream);
    Ok(())
}

pub async fn init_app_state(tcp: &mut TcpStream) -> Result<()> {
    let mut framed_tcp = Framed::new(tcp, LengthDelimitedCodec::new());

    let bytes = match framed_tcp.next().await {
        Some(data) => data.context("failed to load init data from server")?,
        None => {
            bail!("lets bail!")
        }
    };
    let init_data: InitClientData =
        bincode::deserialize(&bytes).context("incorrect init data from server")?;

    println!("{}", init_data.id);

    init_global_state(init_data.id);
    Ok(())
}

pub async fn handle_file_chunk(chunk: Chunk) -> Result<()> {
    let mut state = get_global_state().await;
    let stream_state = state.active_streams.get_mut(&chunk.stream_id).unwrap();
    let bytes_to_write = std::cmp::min(
        chunk.data.len(),
        (stream_state.size - stream_state.written) as usize,
    );
    stream_state.write_all(&chunk.data[0..bytes_to_write]).await;
    stream_state.written += chunk.data.len() as u64;

    println!("{}, {}", stream_state.size, stream_state.written);
    Ok(())
}

pub async fn handle_text_message(msg: TextMessage) -> Result<()> {
    println!("New message from{:?}:  \n {}", msg.from.id, msg.text);
    Ok(())
}

pub async fn write_server(tcp: OwnedWriteHalf, mut rx: mpsc::Receiver<Bytes>) -> Result<()> {
    let mut framed_tcp = FramedWrite::new(tcp, LengthDelimitedCodec::new());
    while let Some(data) = rx.recv().await {
        framed_tcp.send(data).await?;
    }

    Ok(())
}
