use anyhow::{anyhow, bail, Context, Result};
use chat_app::{
    client_lib::{get_global_state, init_global_state},
    shared_lib::{get_addr, Channel, InitClientData, MessageToServer, TextMessage, User},
};
use futures::{SinkExt, StreamExt};
use std::{fs::File, io::Read, str::FromStr, thread, time::Duration};
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::mpsc::{self, Sender},
    task,
};
use tokio_util::{
    bytes::Bytes,
    codec::{Framed, FramedRead, FramedWrite, LengthDelimitedCodec},
};
use uuid::Uuid;

const DEFUALT_HOSTNAME: &str = "localhost";
const DEFUALT_PORT: &str = "11111";

#[tokio::main]
async fn main() -> Result<()> {
    let addr = get_addr(DEFUALT_HOSTNAME, DEFUALT_PORT);

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

            _ => {
                send_text_msg(&mut tcp, &mut buff).await?;
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
            let bytes = frame.context("failed reading framed msg from server")?;

            let msg: MessageToServer =
                bincode::deserialize(&bytes).context("failed bincode reading from server")?;
            match msg {
                MessageToServer::File(_) => {
                    println!("file  chunk arrived");
                }
                MessageToServer::Text(msg) => {
                    println!("New message from{:?}:  \n {}", msg.from.id, msg.text);
                }
            }
        } else {
            tx.send(()).await.unwrap();
            return Err(anyhow!("Server dropped"));
        }
    }
}

async fn send_file(tcp: &mut OwnedWriteHalf, path: &str) -> Result<()> {
    let mut file = File::open(path)?;

    let mut buffer = [0u8; 8192];
    let mut framed_tcp = FramedWrite::new(tcp, LengthDelimitedCodec::new());

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        framed_tcp
            .send(Bytes::copy_from_slice(&buffer[..n]))
            .await?;
    }

    Ok(())
}

// async fn read_file(tcp: &mut OwnedReadHalf) {
//     // let mut file = File::create(save_as).await?;
//     let mut framed_tcp = FramedRead::new(tcp, LengthDelimitedCodec::new());

//     while let Some(chunk_result) = framed_tcp.next().await {
//         let chunk = chunk_result;
//         // file.write_all(&chunk).await?;
//     }
// }

async fn send_text_msg(tcp: &mut OwnedWriteHalf, text: &mut String) -> Result<()> {
    let mut framed_write_tcp = FramedWrite::new(tcp, LengthDelimitedCodec::new());
    let state = get_global_state().await;
    let public_room_id = Uuid::from_str("7e40f106-3e7d-498a-94cc-5fa7f62cfce6").unwrap();

    let text_msg = TextMessage {
        text: text.clone(),
        from: User { id: state.id },
        to: Channel::Room(public_room_id),
    };

    let server_msg = MessageToServer::Text(text_msg);
    let serialized_data =
        bincode::serialize(&server_msg).context("failed bincode writing to server")?;

    framed_write_tcp
        .send(Bytes::from(serialized_data))
        .await
        .context("failed to TCP send msg to server")?;
    Ok(())
}

async fn init_app_state(tcp: &mut TcpStream) -> Result<()> {
    let mut framed_tcp = Framed::new(tcp, LengthDelimitedCodec::new());
    let bytes = match framed_tcp.next().await {
        Some(b) => b.context("faild to load init data from server")?,
        None => {
            bail!("lets bail!")
        }
    };
    let init_data: InitClientData =
        bincode::deserialize(&bytes).context("incorrect init data from server")?;

    init_global_state(init_data.id);
    Ok(())
}
