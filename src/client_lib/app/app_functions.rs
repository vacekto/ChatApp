use crate::{
    client_lib::{
        global_states::app_state::get_app_state,
        util::{config::FILES_DIR, types::ActiveStream},
    },
    shared_lib::types::{Chunk, FileMetadata},
};
use anyhow::Result;
use std::{io::Write, path::Path};

pub fn handle_file_metadata(meta: FileMetadata) -> Result<()> {
    let path = String::from(FILES_DIR) + &meta.name;
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
    };

    let mut state = get_app_state();

    state.active_streams.insert(stream_id, stream);

    Ok(())
}

pub fn handle_file_chunk(chunk: Chunk) -> Result<()> {
    let mut state = get_app_state();
    let stream = state.active_streams.get_mut(&chunk.stream_id).unwrap();
    let bytes_to_write = std::cmp::min(chunk.data.len(), (stream.size - stream.written) as usize);

    stream
        .file_handle
        .write_all(&chunk.data[0..bytes_to_write])?;
    stream.written += chunk.data.len() as u64;

    let written = stream.written;
    let size = stream.size;

    if written == size {
        state.active_streams.remove(&chunk.stream_id).unwrap();
    }

    Ok(())
}

// fn send_file(tx_parse_write: mpsc::Sender<ClientServerMsg>, path: &str) -> Result<()> {
//     let path = Path::new(path);
//     let mut file = std::fs::File::open(path)?;
//     let meta = file.metadata()?;

//     let stream_id = Uuid::new_v4();
//     let mut buffer = [0u8; TCP_CHUNK_BUFFER_SIZE];
//     let state = get_app_state();

//     let meta = FileMetadata {
//         name: String::from(path.file_name().unwrap().to_str().unwrap()),
//         stream_id,
//         to: Channel::Room(Uuid::from_str(PUBLIC_ROOM_ID_STR).unwrap()),
//         size: meta.size(),
//     };

//     let metadata = ClientServerMsg::FileMetadata(meta);

//     tx_parse_write.send(metadata)?;

//     loop {
//         let n = file.read(&mut buffer)?;
//         if n == 0 {
//             break;
//         }
//         let chunk = Chunk {
//             data: buffer.clone(),
//             from: User {
//                 id: state.id,
//                 username: state.username.clone(),
//             },
//             to: Channel::Room(Uuid::from_str(PUBLIC_ROOM_ID_STR).unwrap()),
//             stream_id,
//         };

//         let chunk = ClientServerMsg::FileChunk(chunk);

//         tx_parse_write.send(chunk)?;
//     }
//     Ok(())
// }
