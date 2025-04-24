use std::{collections::HashMap, str::FromStr, vec};

use bytes::{Bytes, BytesMut};
use futures::{SinkExt, StreamExt};
use tokio::{
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    select,
    sync::{
        broadcast::{self, error::RecvError},
        mpsc, oneshot,
    },
    task,
};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use uuid::Uuid;

use crate::{
    server_lib::util::config::log,
    shared_lib::{
        config::{PUBLIC_ROOM_ID_STR, PUBLIC_ROOM_NAME},
        types::{
            Channel, ClientServerMsg, InitClientData, RoomChannel, RoomJoinNotification,
            ServerClientMsg, User,
        },
    },
};

use super::util::{
    config::{COMM_CLIENT_CAPACITY, DIRECT_CAPACITY, MANAGER_CLIENT_CAPACITY},
    errors::DataParsingError,
    server_functions::{serialize_file_chunk, serialize_file_metadata, serialize_text_msg},
    types::{
        ChannelTransitPayload, ClientToManagerMessage, DirectChannelTransit, ManagerToClientMsg,
        MpscChannel,
    },
};

pub struct ClientTask {
    username: String,
    id: Uuid,
    manager: MpscChannel<ClientToManagerMessage, ManagerToClientMsg>,
    comm: MpscChannel,
    rx_room: broadcast::Receiver<Bytes>,
    tcp_read: FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
    tcp_write: FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
    room_channels: HashMap<Uuid, broadcast::Sender<Bytes>>,
    direct_channels: HashMap<Uuid, mpsc::Sender<Bytes>>,
}

impl ClientTask {
    pub async fn new(
        tcp: TcpStream,
        tx_client_manager: mpsc::Sender<ClientToManagerMessage>,
        tx_public_room: broadcast::Sender<Bytes>,
    ) -> Self {
        let id = Uuid::new_v4();

        let (tcp_read, tcp_write) = tcp.into_split();
        let tcp_read = FramedRead::new(tcp_read, LengthDelimitedCodec::new());
        let tcp_write = FramedWrite::new(tcp_write, LengthDelimitedCodec::new());

        let mut room_channels = HashMap::new();
        let direct_channels = HashMap::new();

        room_channels.insert(
            Uuid::from_str(PUBLIC_ROOM_ID_STR).unwrap(),
            tx_public_room.clone(),
        );

        let (tx_manager_client, rx_manager_client) =
            mpsc::channel::<ManagerToClientMsg>(MANAGER_CLIENT_CAPACITY);

        let (tx_comm_client, rx_comm_client) = mpsc::channel::<Bytes>(COMM_CLIENT_CAPACITY);

        tx_client_manager
            .send(ClientToManagerMessage::Init(tx_manager_client, id))
            .await
            .unwrap();

        let comm = MpscChannel {
            tx: tx_comm_client,
            rx: rx_comm_client,
        };

        let manager = MpscChannel {
            tx: tx_client_manager,
            rx: rx_manager_client,
        };

        Self {
            username: "no username provided".into(),
            id,
            direct_channels,
            room_channels,
            rx_room: tx_public_room.subscribe(),
            tcp_read,
            tcp_write,
            comm,
            manager,
        }
    }

    pub async fn run(mut self) {
        loop {
            select! {
                result = self.tcp_read.next() => if let Err(err)= self.handle_tcp_msg(result).await {
                    log(err.into(), None);
                    todo!()
                },
                result = self.manager.rx.recv() => self.handle_manager_msg(result).await,
                result = self.rx_room.recv() => if let Err(err)=self.handle_receive_data(result).await {
                    log(err.into(), None);
                    todo!()
                },
                result = self.comm.rx.recv() => {
                    self.tcp_write.send(result.unwrap()).await.unwrap();
                }
            }
        }
    }

    async fn handle_tcp_msg(
        &mut self,
        result: Option<Result<BytesMut, std::io::Error>>,
    ) -> Result<(), DataParsingError> {
        match result {
            Some(frame) => {
                let data: Bytes = frame.map_err(|err| DataParsingError::from(err))?.into();

                let message: ClientServerMsg =
                    bincode::deserialize(&data).map_err(|err| DataParsingError::from(err))?;

                match message {
                    ClientServerMsg::Text(msg) => {
                        self.send_data(serialize_text_msg(msg.clone())?, msg.to)
                            .await
                    }
                    ClientServerMsg::FileChunk(c) => {
                        self.send_data(serialize_file_chunk(c.clone())?, c.to).await
                    }
                    ClientServerMsg::FileMetadata(m) => {
                        self.send_data(serialize_file_metadata(m.clone())?, m.to)
                            .await
                    }
                    ClientServerMsg::InitClient(username) => self.init_client(username).await?,
                };
            }
            None => {
                todo!("client disconnected, do something. !!!.")
            }
        }
        Ok(())
    }

    async fn handle_manager_msg(&mut self, result: Option<ManagerToClientMsg>) {
        if let Some(t) = result {
            match t {
                ManagerToClientMsg::EstablishDirectComm(c) => {
                    self.direct_channels
                        .insert(c.payload.from, c.payload.tx_client_client);

                    let tx_client_client = self.create_direct_comm_task();
                    c.ack.send(tx_client_client).unwrap();
                }
                ManagerToClientMsg::JoinRoom(c) => {
                    self.room_channels.insert(c.room_id, c.tx.clone());

                    // let rx_client_room = c.tx.subscribe();
                    // create_room_comm_task(rx_client_room, tx_comm_client);
                }
            };
        }
    }

    async fn send_data(&mut self, data: Bytes, target: Channel) {
        match target {
            Channel::User(target_id) => {
                let tx = self.direct_channels.get(&target_id);

                let tx = match tx {
                    Some(tx) => tx,
                    None => {
                        self.establish_direct_comm(target_id, data).await;
                        return;
                    }
                };

                if tx.send(data).await.is_err() {
                    todo!("receivers got dropped, handle after implementing rooms");
                };
            }
            Channel::Room(target_id) => {
                let tx = self
                    .room_channels
                    .get(&target_id)
                    .expect("room not found, implement err handaling");
                println!("found public room");
                if tx.send(data).is_err() {
                    todo!("receivers got dropped, handle after implementing rooms");
                };
            }
        };
    }

    async fn establish_direct_comm(&mut self, target_id: Uuid, data: Bytes) {
        let (tx_ack, rx_ack) = oneshot::channel::<mpsc::Sender<Bytes>>();
        let tx_client_client = self.create_direct_comm_task();

        let channel_transit = DirectChannelTransit {
            payload: ChannelTransitPayload {
                tx_client_client,
                from: self.id,
                to: target_id,
            },
            ack: tx_ack,
        };

        self.manager
            .tx
            .send(ClientToManagerMessage::EstablishDirectComm(channel_transit))
            .await
            .unwrap();

        let new_direct = rx_ack.await.unwrap();
        new_direct.send(data).await.unwrap();

        let direct_channels = &mut self.direct_channels;
        direct_channels.insert(target_id, new_direct);
    }

    fn create_direct_comm_task(&mut self) -> mpsc::Sender<Bytes> {
        let (tx_client_client, mut rx_client_client) = mpsc::channel::<Bytes>(DIRECT_CAPACITY);

        let tx = self.comm.tx.clone();
        task::spawn(async move {
            loop {
                match rx_client_client.recv().await {
                    Some(data) => tx.send(data).await.unwrap(),
                    None => {
                        todo!("other user disconnected, handle...");
                    }
                }
            }
        });

        tx_client_client
    }

    async fn handle_receive_data(
        &mut self,
        result: Result<Bytes, RecvError>,
    ) -> Result<(), DataParsingError> {
        match result {
            Ok(data) => {
                println!("receiving data");
                self.tcp_write.send(data).await?;
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
    async fn init_client(&mut self, username: String) -> Result<(), DataParsingError> {
        self.username = username;

        let public_room = RoomChannel {
            id: Uuid::from_str(PUBLIC_ROOM_ID_STR).unwrap(),
            messages: vec![],
            name: PUBLIC_ROOM_NAME.into(),
            users: vec![],
        };

        let init_data = ServerClientMsg::InitClient(InitClientData {
            id: self.id,
            room_channels: vec![public_room],
        });

        let encoded = bincode::serialize(&init_data)?;
        self.tcp_write.send(encoded.into()).await?;

        for (id, tx_room) in self.room_channels.iter() {
            let msg = ServerClientMsg::UserJoinedRoom(RoomJoinNotification {
                room_id: id.clone(),
                user: User {
                    id: self.id,
                    username: self.username.clone(),
                },
            });

            let bytes = bincode::serialize(&msg)?.into();
            tx_room.send(bytes).unwrap();
        }

        Ok(())
    }
}
