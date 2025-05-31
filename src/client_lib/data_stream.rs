use super::{
    global_states::{app_state::get_global_state, thread_logger::get_thread_runner},
    util::{
        config::{FILES_DIR, FILES_IMG_TO_ASCII},
        types::{ActiveStream, ImgRender, TcpStreamMsg, TuiUpdate},
    },
};
use crate::shared_lib::types::{Chunk, FileMetadata};
use anyhow::Result;
use image::imageops::FilterType;
use std::{collections::HashMap, io::Write, path::Path, sync::mpsc};
use uuid::Uuid;

pub fn handle_file_streaming() -> Result<()> {
    let mut data_streams = HashMap::<Uuid, ActiveStream>::new();
    let mut state = get_global_state();
    let tx_tui = state.tui_update_channel.tx.clone();

    let rx_tcp_stream = state
        .tcp_stream_channel
        .rx
        .take()
        .expect("rx_tcp_stream already taken");

    drop(state);

    while let Ok(msg) = rx_tcp_stream.recv() {
        match msg {
            TcpStreamMsg::FileMetadata(data) => handle_file_metadata(data, &mut data_streams)?,
            TcpStreamMsg::FileChunk(chunk) => {
                handle_file_chunk(chunk, &mut data_streams, tx_tui.clone())?
            }
        }
    }

    Ok(())
}

fn handle_file_chunk(
    chunk: Chunk,
    data_streams: &mut HashMap<Uuid, ActiveStream>,
    tx_tui: mpsc::Sender<TuiUpdate>,
) -> Result<()> {
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
        let name = stream.file_name.clone();
        let suffix = stream.file_name.split(".").last().unwrap();
        if FILES_IMG_TO_ASCII.contains(&suffix) {
            let th = get_thread_runner();
            let from = stream.from.clone();

            th.spawn("image to ascii converter", false, move || {
                let image =
                    image::open(String::from(FILES_DIR) + &name).expect("Failed to open image");
                let resized = image.resize_exact(50, 70, FilterType::Nearest);
                let conf = artem::config::ConfigBuilder::new().color(false).build();
                let ascii = artem::convert(resized, &conf);
                let img_render = ImgRender { cache: ascii, from };
                tx_tui.send(TuiUpdate::Img(img_render))?;

                Ok(())
            });
        }

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
