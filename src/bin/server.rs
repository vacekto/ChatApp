use std::{error::Error, fmt::Debug, io::Error as IoError};

use chat_app::{
    server_lib::log,
    shared_lib::{get_addr, InitClientData},
};
use futures::{SinkExt, StreamExt};
use thiserror::Error;
use tokio::{
    net::{tcp::OwnedWriteHalf, TcpListener, TcpStream},
    select,
    sync::broadcast::{self, error::RecvError, Receiver, Sender},
    task,
};
use tokio_util::{
    bytes::BytesMut,
    codec::{Framed, FramedRead, FramedWrite, LengthDelimitedCodec},
};
use uuid::Uuid;

const DEFUALT_HOSTNAME: &str = "localhost";
const DEFUALT_PORT: &str = "11111";

#[derive(Error, Debug)]
enum DataProcessingError {
    #[error("Failed read/write framed message vie TCP stream, actual error: {0}")]
    FramedTextMessage(#[from] std::io::Error),
    #[error("Failed serialize / deserialize using bincode, actual error: {0}")]
    Bincode(#[from] Box<bincode::ErrorKind>),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = get_addr(DEFUALT_HOSTNAME, DEFUALT_PORT);

    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("listening on: {}", addr);

    let (tx, _) = broadcast::channel::<BytesMut>(30);

    loop {
        match listener.accept().await {
            Ok((tcp, _)) => {
                let tx = tx.clone();
                let rx = tx.subscribe();

                task::spawn(handle_connection(tcp, tx, rx));
            }
            Err(err) => log(err.into(), None),
        }
    }
}

async fn handle_connection(mut tcp: TcpStream, tx: Sender<BytesMut>, mut rx: Receiver<BytesMut>) {
    // use later to subscribe to rooms
    let _ = match init_client(&mut tcp).await {
        Ok(init) => init,
        Err(err) => {
            log(err.into(), Some("failed client initialization"));
            return;
        }
    };

    let (read_tcp, write_tcp) = tcp.into_split();
    let mut read_framed_tcp = FramedRead::new(read_tcp, LengthDelimitedCodec::new());
    let mut write_framed_tcp = FramedWrite::new(write_tcp, LengthDelimitedCodec::new());

    loop {
        select! {
            result = read_framed_tcp.next() =>{
                match data_from_client(result, tx.clone()) {
                    Ok(()) => {},
                    Err(err) => log(err.into(), None)
                };
            }

            result = rx.recv() => {
                match data_to_client(result, &mut write_framed_tcp).await{
                    Ok(()) => {},
                    Err(err) => log(err.into(), None)
                };
            }
        }
    }
}

fn data_from_client(
    result: Option<Result<BytesMut, IoError>>,
    tx: Sender<BytesMut>,
) -> Result<(), DataProcessingError> {
    match result {
        Some(frame) => {
            let bytes = frame?;
            match tx.send(bytes) {
                Ok(_) => {}
                Err(_) => {
                    todo!("receivers got dropped, handle after implementing rooms");
                }
            };
        }
        None => {}
    }
    Ok(())
}

async fn data_to_client(
    result: Result<BytesMut, RecvError>,
    write_framed: &mut FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
) -> Result<(), DataProcessingError> {
    match result {
        Ok(data) => {
            write_framed.send(data.into()).await?;
        }
        Err(broadcast::error::RecvError::Lagged(n)) => {
            println!("Missed {} messages", n);
            todo!("figure out what to do with missed messages");
        }
        Err(broadcast::error::RecvError::Closed) => {
            todo!("senders got dropped, handle after implementing rooms");
        }
    };
    Ok(())
}

async fn init_client(tcp: &mut TcpStream) -> Result<InitClientData, DataProcessingError> {
    let mut framed_tcp = Framed::new(tcp, LengthDelimitedCodec::new());
    let itid_data = InitClientData { id: Uuid::new_v4() };
    let encoded = bincode::serialize(&itid_data)?;
    framed_tcp.send(encoded.into()).await?;
    Ok(itid_data)
}
