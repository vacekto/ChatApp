use anyhow::{anyhow, Context, Result};
use chat_app::{
    client_lib::util_functions::{
        handle_file_chunk, handle_file_metadata, handle_text_message, init_app_state, send_file,
        send_text_msg,
    },
    shared_lib::{
        config::PUBLIC_ROOM_ID_STR,
        types::{Channel, ServerMessage},
        util_functions::get_addr,
    },
};
use futures::StreamExt;
use std::{str::FromStr, thread, time::Duration};
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::mpsc::{self, Sender},
    task,
};
use tokio_util::codec::{FramedRead, LengthDelimitedCodec};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = get_addr("localhost", "11111");

    let mut tcp = loop {
        println!("attempting to establish connection..");
        match TcpStream::connect(&addr).await {
            Ok(s) => {
                println!("connection established with: {}", addr);
                break s;
            }
            Err(e) => {
                println!("connection error: {}", e);
                thread::sleep(Duration::from_secs(1));
            }
        }
    };

    init_app_state(&mut tcp).await?;

    let (read_tcp, write_tcp) = tcp.into_split();
    let (read_tx, mut rx) = mpsc::channel::<()>(20);
    let write_tx = read_tx.clone();

    task::spawn(listen_for_server(read_tcp, read_tx));
    task::spawn(write_to_server(write_tcp, write_tx));

    rx.recv().await;

    println!("finished");

    Ok(())
}

async fn write_to_server(mut tcp: OwnedWriteHalf, tx: Sender<()>) -> Result<()> {
    let mut buff = String::new();
    let mut s_in = BufReader::new(stdin());

    while let Ok(_) = s_in.read_line(&mut buff).await {
        let mut itr = buff.split_whitespace();

        match (itr.next(), itr.next()) {
            (Some(cmd), None) if cmd == ".quit" => {
                break;
            }
            (Some(cmd), Some(_)) if cmd == ".file" && itr.count() != 0 => {
                println!("too many arguments, expected format <>.file> <command>")
            }
            (Some(cmd), Some(path)) if cmd == ".file" => {
                send_file(&mut tcp, path).await?;
            }

            (Some(cmd), Some(id)) => {
                println!("{}", id);
                send_text_msg(
                    &mut tcp,
                    &mut String::from(cmd),
                    Channel::Direct(Uuid::from_str(id.trim()).unwrap()),
                )
                .await?;
            }
            _ => {
                send_text_msg(
                    &mut tcp,
                    &mut buff,
                    Channel::Direct(Uuid::from_str(PUBLIC_ROOM_ID_STR).unwrap()),
                )
                .await?;
            }
        }

        buff.clear();
    }
    tx.send(()).await.unwrap();
    Ok(())
}

async fn listen_for_server(mut tcp: OwnedReadHalf, tx: Sender<()>) -> Result<()> {
    let mut framed = FramedRead::new(&mut tcp, LengthDelimitedCodec::new());

    loop {
        if let Some(frame) = framed.next().await {
            println!("new message");
            let bytes = frame.context("failed reading framed msg from server")?;

            let msg: ServerMessage =
                bincode::deserialize(&bytes).context("failed bincode reading from server")?;

            match msg {
                ServerMessage::FileChunk(chunk) => handle_file_chunk(chunk).await?,
                ServerMessage::Text(msg) => handle_text_message(msg).await?,
                ServerMessage::FileMetadata(meta) => handle_file_metadata(meta).await?,
            }
        } else {
            tx.send(()).await.unwrap();
            return Err(anyhow!("Server dropped"));
        }
    }
}
