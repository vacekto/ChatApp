use std::io::{stdin, BufRead, BufReader, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::str::FromStr;
use std::sync::mpsc;
use std::{os::unix::fs::MetadataExt, path::Path};

use anyhow::{Context, Result};
use uuid::Uuid;

use crate::shared_lib::types::ClientServerMsg;
use crate::shared_lib::{
    config::PUBLIC_ROOM_ID_STR,
    types::{Channel, Chunk, FileMetadata, ServerClientMsg, TextMessage, User},
};

use super::global_states::app_state::{get_app_state, init_app_state};
use super::util::config::{FILES_DIR, TCP_FRAME_SIZE_HEADER};
use super::util::types::ActiveStream;

fn send_file(tx_parse_write: mpsc::Sender<ClientServerMsg>, path: &str) -> Result<()> {
    let path = Path::new(path);
    let mut file = std::fs::File::open(path)?;
    let meta = file.metadata()?;

    let stream_id = Uuid::new_v4();
    let mut buffer = [0u8; 8192];
    let state = get_app_state();

    let meta = FileMetadata {
        name: String::from(path.file_name().unwrap().to_str().unwrap()),
        stream_id,
        to: Channel::Room(Uuid::from_str(PUBLIC_ROOM_ID_STR).unwrap()),
        size: meta.size(),
    };

    let metadata = ClientServerMsg::FileMetadata(meta);

    tx_parse_write.send(metadata)?;

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        let chunk = Chunk {
            data: buffer.clone(),
            from: User { id: state.id },
            to: Channel::Room(Uuid::from_str(PUBLIC_ROOM_ID_STR).unwrap()),
            stream_id,
        };

        let chunk = ClientServerMsg::FileChunk(chunk);

        tx_parse_write.send(chunk)?;
    }
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

pub fn write_server(mut tcp: TcpStream, rx: mpsc::Receiver<ClientServerMsg>) -> Result<()> {
    while let Ok(data) = rx.recv() {
        let serialized = bincode::serialize(&data).context("incorrect init data from server")?;
        let size = serialized.len();

        let mut framed: Vec<u8> = vec![
            (size >> 24) as u8,
            (size >> 16) as u8,
            (size >> 8) as u8,
            size as u8,
        ];

        framed.extend_from_slice(&serialized);

        tcp.write_all(&framed)?;
    }

    Ok(())
}

pub fn read_server(mut tcp: TcpStream) -> Result<()> {
    loop {
        let bytes = read_framed_tcp_msg(&mut tcp)?;
        let message: ServerClientMsg = bincode::deserialize(&bytes)?;
        // println!("new message from server: {:?}", message);

        match message {
            ServerClientMsg::FileChunk(chunk) => handle_file_chunk(chunk)?,
            ServerClientMsg::FileMetadata(meta) => handle_file_metadata(meta)?,
            ServerClientMsg::InitClient(init) => init_app_state(init.id),
            ServerClientMsg::Text(msg) => handle_text_message(msg),
        }
    }
}

fn handle_text_message(msg: TextMessage) {
    println!("new message: {}", msg.text);
}

fn read_framed_tcp_msg(tcp: &mut TcpStream) -> Result<Vec<u8>> {
    let mut size_buf = [0u8; TCP_FRAME_SIZE_HEADER];

    let mut tcp = BufReader::new(tcp);

    match tcp.read_exact(&mut size_buf) {
        Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
            todo!("server dropped... !!.       !");
        }
        Err(e) => return Err(e).context("Error reading TCP size header"),
        _ => {}
    }

    let size = ((size_buf[0] as usize) << 24)
        + ((size_buf[1] as usize) << 16)
        + ((size_buf[2] as usize) << 8)
        + size_buf[3] as usize;

    let mut data = vec![0u8; size];

    tcp.read_exact(&mut data)
        .context("closed connection while reading data of framed message")
        .unwrap();

    Ok(data)
}

pub fn read_stdin(tx: mpsc::Sender<ClientServerMsg>) -> Result<()> {
    let mut buff = String::new();
    let mut s_in = BufReader::new(stdin());

    while let Ok(_) = s_in.read_line(&mut buff) {
        let mut itr = buff.split_whitespace();

        match (itr.next(), itr.next()) {
            (Some(cmd), None) if cmd == ".quit" => {
                break;
            }
            (Some(cmd), Some(_)) if cmd == ".file" && itr.count() != 0 => {
                println!("too many arguments, expected format <>.file> <command>")
            }
            (Some(cmd), Some(path)) if cmd == ".file" => {
                send_file(tx.clone(), path)?;
            }

            (Some(_), Some(id)) => {
                println!("{}", id);
                send_text_msg(
                    buff.clone(),
                    Channel::Direct(Uuid::from_str(id.trim()).unwrap()),
                    tx.clone(),
                )?;
            }
            _ => {
                send_text_msg(
                    buff.clone(),
                    Channel::Room(Uuid::from_str(PUBLIC_ROOM_ID_STR)?),
                    tx.clone(),
                )?;
            }
        }

        buff.clear();
    }
    Ok(())
}

fn send_text_msg(msg: String, to: Channel, tx: mpsc::Sender<ClientServerMsg>) -> Result<()> {
    let state = get_app_state();

    // let state =
    let msg = ClientServerMsg::Text(TextMessage {
        text: msg,
        from: User { id: state.id },
        to,
    });

    tx.send(msg)?;
    Ok(())
}

fn handle_file_metadata(meta: FileMetadata) -> Result<()> {
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
