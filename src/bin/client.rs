use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use chat_app::{
    client_lib::functions::{
        handle_file_chunk, handle_file_metadata, handle_text_message, init_app_state, send_file,
        send_text_msg, write_server,
    },
    server_lib::util::config::{SERVER_HOSTNAME, SERVER_PORT},
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
    net::{tcp::OwnedReadHalf, TcpStream},
    sync::mpsc::{self},
    task,
};
use tokio_util::codec::{FramedRead, LengthDelimitedCodec};
use uuid::Uuid;

#[tokio::main(flavor = "multi_thread", worker_threads = 2f)]
async fn main() -> Result<()> {
    let addr = get_addr(SERVER_HOSTNAME, SERVER_PORT);

    let mut tcp = loop {
        println!("attempting to establish connection..");
        match TcpStream::connect(&addr).await {
            Ok(s) => {
                println!("connection established with: :{}", addr);
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
    let (tx_parse_write, rx_write) = mpsc::channel::<Bytes>(20);

    let read_tcp_task = task::spawn(read_server(read_tcp));
    let write_tcp_task = task::spawn(write_server(write_tcp, rx_write));
    let parse_input_task = task::spawn(parse_input(tx_parse_write));

    tokio::select! {
        res = read_tcp_task => {
            match res {
                Ok(Err(err)) => println!("listen task returned with an error: {}", {err}),
                Ok(Ok(_)) => println!("listen task returned"),
                Err(err) => println!("Error from listen task: {}", err)
            };
        }

        res = write_tcp_task => {
            match res {
                Ok(Err(err)) => println!("write task returned with an error: {}", {err}),
                Ok(_) => println!("write task returned"),
                Err(err) => println!("Error from write task: {}", err)
            };
        }

        res = parse_input_task => {
            match res {
                Ok(Err(err)) => println!("listen task returned with an error: {}", {err}),
                Ok(Ok(_)) => println!("listen task returned"),
                Err(err) => println!("Error from listen task: {}", err)
            };
        }

    }

    println!("program finished");

    Ok(())
}

async fn parse_input(tx_parse_write: mpsc::Sender<Bytes>) -> Result<()> {
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
                send_file(tx_parse_write.clone(), path).await?;
            }

            (Some(cmd), Some(id)) => {
                println!("{}", id);
                send_text_msg(
                    tx_parse_write.clone(),
                    &mut String::from(cmd),
                    Channel::Direct(Uuid::from_str(id.trim()).unwrap()),
                )
                .await?;
            }
            _ => {
                send_text_msg(
                    tx_parse_write.clone(),
                    &mut buff,
                    Channel::Room(Uuid::from_str(PUBLIC_ROOM_ID_STR).unwrap()),
                )
                .await?;
            }
        }

        buff.clear();
    }
    Ok(())
}

async fn read_server(mut tcp: OwnedReadHalf) -> Result<()> {
    let mut framed = FramedRead::new(&mut tcp, LengthDelimitedCodec::new());

    loop {
        if let Some(frame) = framed.next().await {
            let bytes = frame.context("failed reading framed msg from server")?;

            let msg: ServerMessage =
                bincode::deserialize(&bytes).context("failed bincode reading from server")?;

            println!("message from server: {:?}", msg);
            match msg {
                ServerMessage::FileChunk(chunk) => handle_file_chunk(chunk).await?,
                ServerMessage::Text(msg) => handle_text_message(msg).await?,
                ServerMessage::FileMetadata(meta) => handle_file_metadata(meta).await?,
            }
        } else {
            return Err(anyhow!("Server dropped"));
        }
    }
}
