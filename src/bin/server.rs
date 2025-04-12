use chat_app::{
    server_lib::log,
    shared_lib::{get_addr, Channel, InitClientData, MessageToServer, RoomChannel},
};
use futures::TryStreamExt;
use futures::{SinkExt, StreamExt};
use std::{collections::HashMap, error::Error, fmt::Debug, io::Error as IoError, str::FromStr};
use thiserror::Error;
use tokio::{
    net::{tcp::OwnedWriteHalf, TcpListener, TcpStream},
    select,
    sync::{
        broadcast::{self, error::RecvError},
        mpsc,
    },
    task,
};
use tokio_util::{
    bytes::Bytes,
    codec::{Framed, FramedRead, FramedWrite, LengthDelimitedCodec},
};
use uuid::Uuid;

const DEFUALT_HOSTNAME: &str = "localhost";
const DEFUALT_PORT: &str = "11111";
const PUBLIC_ROOM_ID_STR: &str = "7e40f106-3e7d-498a-94cc-5fa7f62cfce6";

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

    let (tx, _) = broadcast::channel::<Bytes>(30);

    // let (tx2, _) = broadcast::channel::<Sender<Bytes>>(30);
    // tx2.send(tx.clone()).unwrap();

    loop {
        match listener.accept().await {
            Ok((tcp, _)) => {
                let tx = tx.clone();
                let rx = tx.subscribe();
                let public_room = RoomChannel { tx, rx };
                task::spawn(handle_connection(tcp, public_room));
            }
            Err(err) => log(err.into(), None),
        }
    }
}

async fn handle_connection(mut tcp: TcpStream, mut public_room: RoomChannel) {
    // TODO: extend to fetch user data
    let _ = match init_client(&mut tcp).await {
        Ok(init) => init,
        Err(err) => {
            log(err.into(), Some("failed client initialization"));
            return;
        }
    };

    let mut room_channels: HashMap<Uuid, broadcast::Sender<Bytes>> = HashMap::new();
    let direct_channels: HashMap<Uuid, mpsc::Sender<Bytes>> = HashMap::new();

    let public_room_id = Uuid::from_str(PUBLIC_ROOM_ID_STR).unwrap();
    room_channels.insert(public_room_id, public_room.tx.clone());

    let (read_tcp, write_tcp) = tcp.into_split();
    let mut read_framed_tcp =
        FramedRead::new(read_tcp, LengthDelimitedCodec::new()).map_ok(|b| b.freeze());
    let mut write_framed_tcp = FramedWrite::new(write_tcp, LengthDelimitedCodec::new());

    loop {
        select! {
            result = read_framed_tcp.next() =>{
                match data_from_client(result, &room_channels, &direct_channels).await {
                    Ok(()) => {},
                    Err(err) => log(err.into(), None)
                };
            }

            result = public_room.rx.recv() => {
                match data_to_client(result, &mut write_framed_tcp).await{
                    Ok(()) => {},
                    Err(err) => log(err.into(), None)
                };
            }
        }
    }
}

async fn data_from_client(
    result: Option<Result<Bytes, IoError>>,
    room_channels: &HashMap<Uuid, broadcast::Sender<Bytes>>,
    direct_channels: &HashMap<Uuid, mpsc::Sender<Bytes>>,
) -> Result<(), DataProcessingError> {
    match result {
        Some(frame) => {
            let bytes = frame?;

            let message: MessageToServer = bincode::deserialize(&bytes)?;

            match message {
                MessageToServer::File(_) => {
                    todo!()
                }
                MessageToServer::Text(msg) => match msg.to {
                    Channel::Direct(id) => {
                        let tx = direct_channels.get(&id).unwrap();
                        if tx.send(bytes).await.is_err() {
                            todo!("receivers got dropped, handle after implementing rooms");
                        };
                    }
                    Channel::Room(id) => {
                        let tx = room_channels.get(&id).unwrap();
                        if tx.send(bytes).is_err() {
                            todo!("receivers got dropped, handle after implementing rooms");
                        };
                    }
                },
            };
        }
        None => {}
    }
    Ok(())
}

async fn data_to_client(
    result: Result<Bytes, RecvError>,
    write_framed: &mut FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
) -> Result<(), DataProcessingError> {
    match result {
        Ok(data) => {
            write_framed.send(data).await?;
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
