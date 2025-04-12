use futures::{SinkExt, StreamExt};
use std::path::Path;
use std::str::FromStr;

use anyhow::{bail, Context, Result};
use bytes::Bytes;
use tokio::{
    fs::File,
    io::AsyncReadExt,
    net::{tcp::OwnedWriteHalf, TcpStream},
};
use tokio_util::codec::{Framed, FramedWrite, LengthDelimitedCodec};
use uuid::Uuid;

use crate::shared_lib::types::{
    Channel, Chunk, FileMetadata, InitClientData, ServerMessage, TextMessage, User,
};

use super::{
    app_state::{get_global_state, init_global_state},
    types::ActiveStream,
};

pub async fn send_file(tcp: &mut OwnedWriteHalf, path: &str) -> Result<()> {
    let path = Path::new(path);
    let stream_id = Uuid::new_v4();
    let mut file = File::open(path).await?;
    let mut buffer = [0u8; 8192];
    let mut framed_tcp = FramedWrite::new(tcp, LengthDelimitedCodec::new());
    let state = get_global_state().await;

    let meta = file.metadata().await?;

    let meta = FileMetadata {
        size: meta.len(),
        name: String::from(path.file_name().unwrap().to_str().unwrap()),
        stream_id,
        to: Channel::Room(Uuid::from_str("7e40f106-3e7d-498a-94cc-5fa7f62cfce6").unwrap()),
    };

    let server_message = ServerMessage::FileMetadata(meta);

    let server_message = bincode::serialize(&server_message)?;

    framed_tcp
        .send(Bytes::copy_from_slice(&server_message))
        .await?;

    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        let chunk = Chunk {
            data: &buffer[0..n],
            from: User { id: state.id },
            to: Channel::Room(Uuid::from_str("7e40f106-3e7d-498a-94cc-5fa7f62cfce6").unwrap()),
            stream_id,
        };

        let msg = ServerMessage::File(chunk);

        let serialized =
            bincode::serialize(&msg).context("bincode failed to serialize file chunk")?;

        framed_tcp.send(Bytes::copy_from_slice(&serialized)).await?;
    }
    Ok(())
}

pub async fn send_text_msg(tcp: &mut OwnedWriteHalf, text: &mut String) -> Result<()> {
    let mut framed_write_tcp = FramedWrite::new(tcp, LengthDelimitedCodec::new());
    let state = get_global_state().await;
    let public_room_id = Uuid::from_str("7e40f106-3e7d-498a-94cc-5fa7f62cfce6").unwrap();

    let text_msg = TextMessage {
        text: text.clone(),
        from: User { id: state.id },
        to: Channel::Room(public_room_id),
    };

    let server_msg = ServerMessage::Text(text_msg);
    let serialized_data =
        bincode::serialize(&server_msg).context("failed bincode writing to server")?;

    framed_write_tcp
        .send(Bytes::from(serialized_data))
        .await
        .context("failed to TCP send msg to server")?;
    Ok(())
}

pub async fn handle_file_metadata(meta: FileMetadata) -> Result<()> {
    let file: File = File::create(String::from("./files/") + &meta.name).await?;
    let mut state = get_global_state().await;

    let stream = ActiveStream {
        file_handle: file,
        size: meta.size,
        written: 0,
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

    init_global_state(init_data.id);
    Ok(())
}

pub async fn handle_file_chunk<'a>(chunk: Chunk<'a>) -> Result<()> {
    let mut state = get_global_state().await;
    let stream = state.active_streams.get_mut(&chunk.stream_id).unwrap();
    stream.write_all(&chunk.data).await;
    stream.written += chunk.data.len() as u64;
    println!("written: {}, size: {}", stream.written, stream.size);
    Ok(())
}

pub async fn handle_text_message(msg: TextMessage) -> Result<()> {
    println!("New message from{:?}:  \n {}", msg.from.id, msg.text);
    Ok(())
}
