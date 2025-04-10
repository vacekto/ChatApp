use anyhow::{anyhow, bail, Context, Result};
use chat_app::{
    client_lib::init_global_state,
    shared_lib::{get_addr, InitClientData, MsgMetadata, TextMessage},
};
use futures::{SinkExt, StreamExt};
use std::{fs::File, io::Read, thread, time::Duration};
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
    bytes::{Bytes, BytesMut},
    codec::{Framed, FramedRead, FramedWrite, LengthDelimitedCodec},
};

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
            let msg: TextMessage =
                bincode::deserialize(&bytes).context("failed bincode reading from server")?;
            println!("{:?}", msg);
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

async fn read_file(tcp: &mut OwnedReadHalf) {
    // let mut file = File::create(save_as).await?;
    let mut framed_tcp = FramedRead::new(tcp, LengthDelimitedCodec::new());

    while let Some(chunk_result) = framed_tcp.next().await {
        let chunk = chunk_result;
        // file.write_all(&chunk).await?;
    }
}

fn send_whole_data(tcp: &mut OwnedWriteHalf) {}

fn read_data(bytes: BytesMut) {
    let mut meta_data: MsgMetadata;
    // if let Ok(meta) = bincode::deserialize(&bytes).context("failed bincode reading from server")?;
}

async fn send_text_msg(tcp: &mut OwnedWriteHalf, msg: &mut String) -> Result<()> {
    let mut framed_write_tcp = FramedWrite::new(tcp, LengthDelimitedCodec::new());

    let meta = MsgMetadata {
        sender: String::from("cosikdosi"),
        size: msg.len(),
    };
    let data = TextMessage(msg.clone());

    let serialized_meta = bincode::serialize(&meta).context("failed bincode writing to server")?;
    let serialized_data = bincode::serialize(&data).context("failed bincode writing to server")?;

    framed_write_tcp
        .send(Bytes::from(serialized_meta))
        .await
        .context("failed to TCP send msg to server")?;

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
            bail!("lets bail baby!")
        }
    };
    let init_data: InitClientData =
        bincode::deserialize(&bytes).context("incorrect init data from server")?;

    init_global_state(init_data.id);
    Ok(())
}
