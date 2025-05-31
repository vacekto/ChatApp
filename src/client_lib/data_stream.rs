use super::{
    global_states::app_state::get_global_state,
    util::{
        config::FILES_DIR,
        types::{ActiveStream, TcpStreamMsg},
    },
};
use crate::shared_lib::types::{Chunk, FileMetadata};
use anyhow::Result;
use std::{collections::HashMap, io::Write, path::Path};
use uuid::Uuid;

pub fn handle_file_streaming() -> Result<()> {
    let mut data_streams = HashMap::<Uuid, ActiveStream>::new();
    let mut state = get_global_state();

    let rx_tcp_stream = state
        .tcp_stream_channel
        .rx
        .take()
        .expect("rx_tcp_stream already taken");

    drop(state);

    while let Ok(msg) = rx_tcp_stream.recv() {
        match msg {
            TcpStreamMsg::FileMetadata(data) => handle_file_metadata(data, &mut data_streams)?,
            TcpStreamMsg::FileChunk(chunk) => handle_file_chunk(chunk, &mut data_streams)?,
        }
    }

    Ok(())
}

fn handle_file_chunk(chunk: Chunk, data_streams: &mut HashMap<Uuid, ActiveStream>) -> Result<()> {
    let stream = match data_streams.get_mut(&chunk.stream_id) {
        Some(s) => s,
        None => return Ok(()),
    };
    let bytes_to_write = std::cmp::min(chunk.data.len(), (stream.size - stream.written) as usize);

    stream
        .file_handle
        .write_all(&chunk.data[0..bytes_to_write])?;
    stream.written += bytes_to_write as u64;

    let written = stream.written;
    let size = stream.size;

    if written == size {
        data_streams.remove(&chunk.stream_id).unwrap();
    }

    Ok(())
}

pub fn handle_file_metadata(
    meta: FileMetadata,
    data_streams: &mut HashMap<Uuid, ActiveStream>,
) -> Result<()> {
    let path = String::from(FILES_DIR) + &meta.filename;
    let path = Path::new(&path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = std::fs::File::create(path)?;
    let stream_id = meta.stream_id;

    let stream = ActiveStream {
        file_handle: file,
        size: meta.size,
        written: 0,
        file_name: meta.filename,
        from: meta.from,
    };

    data_streams.insert(stream_id, stream);

    Ok(())
}
