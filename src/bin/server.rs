use std::{error::Error, fmt::Debug};

use chat_app::{
    server_lib::log,
    shared_lib::{get_addr, TextMessage},
};
use futures::{SinkExt, StreamExt};
use thiserror::Error;
use tokio::{
    net::{TcpListener, TcpStream},
    select,
    sync::broadcast::{self, Receiver, Sender},
    task,
};
use tokio_util::{
    bytes::Bytes,
    codec::{Framed, LengthDelimitedCodec},
};

const DEFUALT_HOSTNAME: &str = "localhost";
const DEFUALT_PORT: &str = "11111";

#[derive(Error, Debug)]
enum DataProcessingError {
    #[error("Failed read/write framed message vie TCP stream")]
    FramedTextMessage(std::io::Error),
    #[error("bincoud")]
    Bincode(bincode::ErrorKind),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = get_addr(DEFUALT_HOSTNAME, DEFUALT_PORT);

    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("listening on: {}", addr);

    let (tx, _) = broadcast::channel::<TextMessage>(30);

    loop {
        match listener.accept().await {
            Ok((tcp, _)) => {
                let tx = tx.clone();
                let rx = tx.subscribe();

                task::spawn(handle_connection(tcp, tx, rx));
            }
            Err(err) => log(Box::new(err), None),
        }
    }
}

async fn handle_connection(
    mut tcp: TcpStream,
    tx: Sender<TextMessage>,
    mut rx: Receiver<TextMessage>,
) {
    let mut framed = Framed::new(&mut tcp, LengthDelimitedCodec::new());

    loop {
        select! {
            result = framed.next() =>{
                if let Some(frame) = result {
                    let bytes = match frame {
                        Ok(b) => b,
                        Err(er) => {
                            log(Box::new(DataProcessingError::FramedTextMessage(er)), None);
                            continue;
                        }
                    };
                    let msg: TextMessage = match bincode::deserialize(&bytes) {
                        Ok(msg) => msg,
                        Err(err) => {
                            log(Box::new(DataProcessingError::Bincode(*err)), None);
                            continue;
                        }
                    };
                    println!("{:?}", msg);
                    match tx.send(msg) {
                        Ok(_) => {}
                        Err(_) => {
                            todo!("no one listening, buffer messages to resend on client reconnect")
                        }
                    };
                }
            }

            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        let s = match bincode::serialize(&msg) {
                            Ok(msg) => msg,
                            Err(err) => {
                                log(Box::new(DataProcessingError::Bincode(*err)), None);
                                continue;
                            }
                        };
                        match framed.send(Bytes::from(s)).await {
                            Ok(b) => b,
                            Err(er) => {
                                log(Box::new(DataProcessingError::FramedTextMessage(er)), None);
                                continue;
                            }
                        };
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        println!("Missed {} messages", n);
                        todo!("figure out what to do with missed messages from broadcast channel");
                    }
                    Err(_) => break,
                }
            }
        }
    }
}
