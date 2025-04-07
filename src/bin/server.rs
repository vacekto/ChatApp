use chat_app::shared_lib::{get_addr, BUFF_LENGTH};
use std::{collections::HashMap, io::Write};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    select,
    sync::broadcast::{self, Receiver, Sender},
    task,
};
use uuid::Uuid;

const DEFUALT_HOSTNAME: &str = "localhost";
const DEFUALT_PORT: &str = "11111";

type Message = String;

#[tokio::main]
async fn main() {
    let addr = get_addr(DEFUALT_HOSTNAME, DEFUALT_PORT);

    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("listening on: {}", addr);

    let (tx, _) = broadcast::channel::<Message>(20);

    loop {
        let (tcp, _) = listener.accept().await.unwrap();
        let tx = tx.clone();
        let rx = tx.subscribe();

        task::spawn(handle_connection(tcp, tx, rx));
    }
}

async fn handle_connection(mut tcp: TcpStream, tx: Sender<Message>, mut rx: Receiver<Message>) {
    let mut buff: [u8; BUFF_LENGTH] = [0; BUFF_LENGTH];

    loop {
        select! {
            result = tcp.read(&mut buff) => {
                match result {
                    Ok(0) => {
                        println!("Client disconnected");
                        break;
                    }
                    Ok(n) => {
                        let msg = String::from_utf8_lossy(&buff[..n]);
                        tx.send(msg.to_string()).unwrap();
                    }
                    Err(e) => {
                        println!("TCP error: {}", e);
                        break;
                    }
                }
            }

            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        println!("Received broadcast: ");
                        tcp.write_all(msg.as_bytes()).await.unwrap();
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        println!("Missed {} messages", n);
                    }
                    Err(_) => break,
                }
            }
        }
    }
}
