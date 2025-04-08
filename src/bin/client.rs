use chat_app::shared_lib::{get_addr, TextMessage};
use futures::{SinkExt, StreamExt};
use std::error::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc::{self, Sender}; // <- this is key
use tokio::{
    io::stdin,
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    task,
};
use tokio_util::{
    bytes::Bytes,
    codec::{FramedRead, FramedWrite, LengthDelimitedCodec},
};

const DEFUALT_HOSTNAME: &str = "localhost";
const DEFUALT_PORT: &str = "11111";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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
                // thread::sleep(Duration::from_secs(2));
            }
        }
    };

    let (in_tcp, out_tcp) = tcp.into_split();
    let (in_tx, mut rx) = mpsc::channel::<()>(20);
    let out_tx = in_tx.clone();

    task::spawn(listen_for_server(in_tcp, in_tx));
    task::spawn(write_to_server(out_tcp, out_tx));

    rx.recv().await.unwrap();

    println!("finished");

    Ok(())
}

async fn write_to_server(mut tcp: OwnedWriteHalf, tx: Sender<()>) {
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
                send_text_msg(&mut tcp, &mut buff).await;
            }
        }

        buff.clear();
    }
    tx.send(()).await.unwrap();
}

async fn listen_for_server(mut tcp: OwnedReadHalf, tx: Sender<()>) {
    let mut framed = FramedRead::new(&mut tcp, LengthDelimitedCodec::new());

    loop {
        if let Some(frame) = framed.next().await {
            let bytes = frame.unwrap();
            let msg: TextMessage = bincode::deserialize(&bytes).unwrap();
            println!("{:?}", msg);
        } else {
            println!("Connection closed");
            break;
        }
    }
    tx.send(()).await.unwrap();
}

fn send_file(_: &mut OwnedWriteHalf, _: &str) {
    unimplemented!();
}

async fn send_text_msg(tcp: &mut OwnedWriteHalf, msg: &mut String) {
    let msg = TextMessage {
        sender: String::from("cosikdosi"),
        content: msg.clone(),
    };
    let mut framed = FramedWrite::new(tcp, LengthDelimitedCodec::new());
    let s = bincode::serialize(&msg).unwrap();
    framed.send(Bytes::from(s)).await.unwrap();
}
