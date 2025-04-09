use anyhow::{anyhow, Context, Result};
use chat_app::shared_lib::{get_addr, TextMessage};
use futures::{SinkExt, StreamExt};
use std::{thread, time::Duration};
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
    codec::{FramedRead, FramedWrite, LengthDelimitedCodec},
};

const DEFUALT_HOSTNAME: &str = "localhost";
const DEFUALT_PORT: &str = "11111";

#[tokio::main]
async fn main() -> Result<()> {
    let addr = get_addr(DEFUALT_HOSTNAME, DEFUALT_PORT);
    let tcp = loop {
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

    let (in_tcp, out_tcp) = tcp.into_split();
    let (in_tx, mut rx) = mpsc::channel::<()>(20);
    let out_tx = in_tx.clone();

    task::spawn(listen_for_server(in_tcp, in_tx));
    task::spawn(write_to_server(out_tcp, out_tx));

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
                send_file(&mut tcp, path);
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

fn send_file(_: &mut OwnedWriteHalf, _: &str) {
    unimplemented!();
}

async fn send_text_msg(tcp: &mut OwnedWriteHalf, msg: &mut String) -> Result<()> {
    let msg = TextMessage {
        sender: String::from("cosikdosi"),
        content: msg.clone(),
    };
    let mut framed = FramedWrite::new(tcp, LengthDelimitedCodec::new());
    let s = bincode::serialize(&msg).context("failed bincode writing to server")?;
    framed
        .send(Bytes::from(s))
        .await
        .context("failed to send msg to server")?;
    Ok(())
}
