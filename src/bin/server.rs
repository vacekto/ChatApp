use chat_app::server_lib::{
    config::{log, DEFUALT_HOSTNAME, DEFUALT_PORT},
    errors::{ChannelNotFoundError, DataProcessingError, MessageTransitError},
    types::{
        ChannelTransit, ChannelTransitPayload, ClientToManagerMessage, ManagerToClientMessage,
    },
};
use chat_app::shared_lib::{
    config::PUBLIC_ROOM_ID_STR,
    types::{Channel, InitClientData, ServerMessage},
    util_functions::get_addr,
};
use futures::{SinkExt, StreamExt, TryStreamExt};
use std::{collections::HashMap, error::Error, io::Error as IoError, str::FromStr};
use tokio::{
    net::{tcp::OwnedWriteHalf, TcpListener, TcpStream},
    select,
    sync::{
        broadcast::{self, error::RecvError},
        mpsc, oneshot,
    },
    task,
};
use tokio_util::{
    bytes::Bytes,
    codec::{Framed, FramedRead, FramedWrite, LengthDelimitedCodec},
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = get_addr(DEFUALT_HOSTNAME, DEFUALT_PORT);

    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("listening on: {}", addr);

    // public room broadcast
    let (tx_public_room, _) = broadcast::channel::<Bytes>(30);

    let (tx_client_manager, rx_client_manager) = mpsc::channel::<ClientToManagerMessage>(30);

    create_manager_task(rx_client_manager);

    loop {
        match listener.accept().await {
            Ok((tcp, _)) => {
                let tx_public_room = tx_public_room.clone();
                let rx_public_room = tx_public_room.subscribe();
                let tx_client_manager = tx_client_manager.clone();

                task::spawn(handle_connection(
                    tcp,
                    tx_client_manager,
                    tx_public_room,
                    rx_public_room,
                ));
            }
            Err(err) => log(err.into(), None),
        }
    }
}

async fn handle_connection(
    mut tcp: TcpStream,
    tx_client_manager: mpsc::Sender<ClientToManagerMessage>,
    tx_public_room: broadcast::Sender<Bytes>,
    mut rx_public_room: broadcast::Receiver<Bytes>,
) {
    // TODO: extend to fetch user data
    let client_data = match init_client(&mut tcp).await {
        Ok(init) => init,
        Err(err) => {
            log(err.into(), Some("failed client initialization"));
            return;
        }
    };

    let (tx_manager_client, mut rx_manager_client) = mpsc::channel::<ManagerToClientMessage>(30);

    tx_client_manager
        .send(ClientToManagerMessage::Init(
            tx_manager_client,
            client_data.id,
        ))
        .await
        .unwrap();

    let mut room_channels: HashMap<Uuid, broadcast::Sender<Bytes>> = HashMap::new();
    let mut direct_channels: HashMap<Uuid, mpsc::Sender<Bytes>> = HashMap::new();

    let public_room_id = Uuid::from_str(PUBLIC_ROOM_ID_STR).unwrap();
    room_channels.insert(public_room_id, tx_public_room.clone());

    let (read_tcp, write_tcp) = tcp.into_split();
    let mut read_framed_tcp =
        FramedRead::new(read_tcp, LengthDelimitedCodec::new()).map_ok(|b| b.freeze());
    let mut write_framed_tcp = FramedWrite::new(write_tcp, LengthDelimitedCodec::new());

    let (tx_comm_client, mut rx_comm_client) = mpsc::channel::<Bytes>(30);

    loop {
        select! {
            result = read_framed_tcp.next() => {
                 if let Err(err) = handle_send_data(result, &room_channels, &direct_channels).await {
                    handle_message_transit_error(err, tx_comm_client.clone(), tx_client_manager.clone(), client_data.id,  &mut direct_channels).await;
                };
            }

            result = rx_manager_client.recv() => {
                handle_manager(result, &mut direct_channels,  tx_comm_client.clone());

            }

            result = rx_public_room.recv() => {
                match handle_receive_data(result, &mut write_framed_tcp).await{
                    Ok(()) => {},
                    Err(err) => log(err.into(), None)
                };
            }


            result = rx_comm_client.recv() => {
                write_framed_tcp.send(result.unwrap()).await.unwrap();
            }

        }
    }
}

async fn handle_send_data(
    result: Option<Result<Bytes, IoError>>,
    room_channels: &HashMap<Uuid, broadcast::Sender<Bytes>>,
    direct_channels: &HashMap<Uuid, mpsc::Sender<Bytes>>,
) -> Result<(), MessageTransitError> {
    match result {
        Some(frame) => {
            let bytes = frame.map_err(|err| DataProcessingError::from(err))?;
            let message: ServerMessage =
                bincode::deserialize(&bytes).map_err(|err| DataProcessingError::from(err))?;
            match message {
                ServerMessage::Text(msg) => {
                    send_data(room_channels, direct_channels, msg.to, bytes).await?
                }

                ServerMessage::FileChunk(c) => {
                    send_data(room_channels, direct_channels, c.to, bytes).await?
                }

                ServerMessage::FileMetadata(m) => {
                    send_data(room_channels, direct_channels, m.to, bytes).await?
                }
            };
        }
        None => {}
    }
    Ok(())
}

async fn handle_receive_data(
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
    let itid_data = InitClientData { id: Uuid::new_v4() };

    let mut framed_tcp = Framed::new(tcp, LengthDelimitedCodec::new());
    let encoded = bincode::serialize(&itid_data)?;
    framed_tcp.send(encoded.into()).await?;
    Ok(itid_data)
}

async fn send_data(
    room_channels: &HashMap<Uuid, broadcast::Sender<Bytes>>,
    direct_channels: &HashMap<Uuid, mpsc::Sender<Bytes>>,
    target: Channel,
    data: Bytes,
) -> Result<(), ChannelNotFoundError> {
    match target {
        Channel::Direct(id) => {
            let tx = direct_channels
                .get(&id)
                .ok_or(ChannelNotFoundError::Direct(id, data.clone()).into())?;

            if tx.send(data).await.is_err() {
                todo!("receivers got dropped, handle after implementing rooms");
            };
        }
        Channel::Room(id) => {
            let tx = room_channels
                .get(&id)
                .ok_or(ChannelNotFoundError::Room(id, data.clone()).into())?;
            if tx.send(data).is_err() {
                todo!("receivers got dropped, handle after implementing rooms");
            };
        }
    };
    Ok(())
}

async fn handle_message_transit_error(
    err: MessageTransitError,
    tx_comm_client: mpsc::Sender<Bytes>,
    tx_client_manager: mpsc::Sender<ClientToManagerMessage>,
    client_id: Uuid,
    direct_channels: &mut HashMap<Uuid, mpsc::Sender<Bytes>>,
) {
    match err {
        MessageTransitError::Recoverable(err) => match err {
            ChannelNotFoundError::Direct(target_id, msg) => {
                let (tx_client_client, rx_client_client) = mpsc::channel::<Bytes>(30);

                let (tx_ack, rx_ack) = oneshot::channel::<mpsc::Sender<Bytes>>();
                tx_client_manager
                    .send(ClientToManagerMessage::EstablishDirectComm(
                        ChannelTransit {
                            payload: ChannelTransitPayload {
                                tx_client_client,
                                from: client_id,
                                to: target_id,
                            },
                            ack: tx_ack,
                        }, //(tx_client_client, client_id)
                    ))
                    .await
                    .unwrap();
                let new_direct = rx_ack.await.unwrap();
                new_direct.send(msg).await.unwrap();
                direct_channels.insert(target_id, new_direct);
                create_comm_task(rx_client_client, tx_comm_client);
            }
            ChannelNotFoundError::Room(..) => {
                todo!()
            }
        },
        MessageTransitError::Unrecoverable(err) => {
            log(err.into(), None);
        }
    }
}
// tx_comm_client.clone(), tx_client_manager.clone()

fn create_comm_task(
    mut rx_client_client: mpsc::Receiver<Bytes>,
    tx_comm_client: mpsc::Sender<Bytes>,
) {
    task::spawn(async move {
        while let Some(data) = rx_client_client.recv().await {
            tx_comm_client.send(data).await.unwrap();
        }
    });
}

fn handle_manager(
    result: Option<ManagerToClientMessage>,
    direct_channels: &mut HashMap<Uuid, mpsc::Sender<Bytes>>,
    tx_comm_client: mpsc::Sender<Bytes>,
) {
    if let Some(t) = result {
        match t {
            ManagerToClientMessage::EstablishDirectComm(c) => {
                direct_channels.insert(c.payload.from, c.payload.tx_client_client);

                let (tx_client_client, rx_client_client) = mpsc::channel::<Bytes>(30);

                create_comm_task(rx_client_client, tx_comm_client);
                c.ack.send(tx_client_client).unwrap();
            }
        };
    }
}

fn create_manager_task(mut rx_client_manager: mpsc::Receiver<ClientToManagerMessage>) {
    task::spawn(async move {
        let mut clients: HashMap<Uuid, mpsc::Sender<ManagerToClientMessage>> = HashMap::new();

        loop {
            match rx_client_manager.recv().await.unwrap() {
                ClientToManagerMessage::Init(tx_manager_client, id) => {
                    clients.insert(id, tx_manager_client);
                }
                ClientToManagerMessage::ClientDropped(id) => {
                    clients.remove(&id);
                }
                ClientToManagerMessage::EstablishDirectComm(c) => {
                    let tx_target = clients.get(&c.payload.to).unwrap();
                    let (tx_ack, rx_ack) = oneshot::channel::<mpsc::Sender<Bytes>>();

                    let transit = ChannelTransit {
                        ack: tx_ack,
                        payload: c.payload,
                    };

                    tx_target
                        .send(ManagerToClientMessage::EstablishDirectComm(transit))
                        .await
                        .unwrap();
                    let tx_cleint_client = rx_ack.await.unwrap();
                    c.ack.send(tx_cleint_client).unwrap();
                }
            };
        }
    });
}
