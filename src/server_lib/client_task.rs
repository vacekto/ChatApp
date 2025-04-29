use std::collections::HashMap;

use bytes::{Bytes, BytesMut};
use futures::{SinkExt, StreamExt};
use tokio::{
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    select,
    sync::{
        broadcast::{self},
        mpsc, oneshot,
    },
    task,
};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use uuid::Uuid;

use crate::{
    server_lib::util::config::log,
    shared_lib::types::{Channel, ClientServerMsg, InitClientData, ServerClientMsg, User},
};

use super::util::{
    config::{COMM_CLIENT_CAPACITY, DIRECT_CAPACITY, MANAGER_CLIENT_CAPACITY},
    errors::DataParsingError,
    server_functions::{serialize_file_chunk, serialize_file_metadata, serialize_text_msg},
    types::{
        ChannelTransitPayload, Client, ClientManagerMsg, DirectChannelTransit, ManagerClientMsg,
        MpscChannel,
    },
};

pub struct ClientTask {
    _username: String,
    id: Uuid,
    manager: MpscChannel<ClientManagerMsg, ManagerClientMsg>,
    comm: MpscChannel,
    tcp_read: FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
    tcp_write: FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
    room_channels: HashMap<Uuid, broadcast::Sender<Bytes>>,
    direct_channels: HashMap<Uuid, mpsc::Sender<Bytes>>,
    connected: bool,
}

impl ClientTask {
    pub async fn new(
        init_data: InitClientData,
        tcp_read: FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
        tcp_write: FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
        tx_client_manager: mpsc::Sender<ClientManagerMsg>,
    ) -> Self {
        let id = init_data.id;

        let room_channels = HashMap::new();
        let direct_channels = HashMap::new();

        let (tx_manager_client, rx_manager_client) =
            mpsc::channel::<ManagerClientMsg>(MANAGER_CLIENT_CAPACITY);

        let client = Client {
            tx: tx_manager_client,
            user: User {
                username: init_data.username.clone(),
                id,
            },
        };

        tx_client_manager
            .send(ClientManagerMsg::Init(client))
            .await
            .unwrap();

        let (tx_comm_client, rx_comm_client) = mpsc::channel::<Bytes>(COMM_CLIENT_CAPACITY);

        let comm = MpscChannel {
            tx: tx_comm_client,
            rx: rx_comm_client,
        };

        let manager = MpscChannel {
            tx: tx_client_manager,
            rx: rx_manager_client,
        };

        Self {
            _username: init_data.username,
            id,
            direct_channels,
            room_channels,
            tcp_read,
            tcp_write,
            comm,
            manager,
            connected: true,
        }
    }

    pub async fn run(mut self) {
        while self.connected {
            select! {
                result = self.tcp_read.next() => if let Err(err)= self.handle_tcp_msg(result).await {
                    log(err.into(), None);
                    todo!()
                },
                result = self.manager.rx.recv() => if let Err(err )=self.handle_manager_msg(result).await{
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
                };
            }
            None => {
                let msg = ClientManagerMsg::ClientDropped(self.id);
                self.manager.tx.send(msg).await.unwrap();
                self.connected = false
            }
        }
        Ok(())
    }

    async fn handle_manager_msg(
        &mut self,
        result: Option<ManagerClientMsg>,
    ) -> Result<(), DataParsingError> {
        if let Some(msg) = result {
            match msg {
                ManagerClientMsg::EstablishDirectComm(c) => {
                    self.direct_channels
                        .insert(c.payload.from, c.payload.tx_client_client);

                    let tx_client_client = self.create_direct_communication_task();
                    c.ack.send(tx_client_client).unwrap();
                }
                ManagerClientMsg::JoinRoom(transit) => {
                    let room_id = transit.room.id;
                    self.room_channels.insert(room_id, transit.tx.clone());

                    let msg = ServerClientMsg::JoinRoom(transit.room.clone());
                    let bytes = bincode::serialize(&msg)?;
                    self.tcp_write.send(bytes.into()).await?;

                    let tx = self.create_room_communication_task(transit.tx);

                    let msg = ServerClientMsg::RoomUpdate(transit.room);

                    let bytes = bincode::serialize(&msg)?.into();
                    tx.send(bytes).unwrap();
                }
            };
        }
        Ok(())
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
                    todo!("direct receiver got dropped");
                };
            }
            Channel::Room(target_id) => {
                let tx = self
                    .room_channels
                    .get(&target_id)
                    .expect("room not found, implement err handaling");
                if tx.send(data).is_err() {
                    todo!("room receivers got dropped");
                };
            }
        };
    }

    async fn establish_direct_comm(&mut self, target_id: Uuid, data: Bytes) {
        let (tx_ack, rx_ack) = oneshot::channel::<mpsc::Sender<Bytes>>();
        let tx_client_client = self.create_direct_communication_task();

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
            .send(ClientManagerMsg::EstablishDirectComm(channel_transit))
            .await
            .unwrap();

        let new_direct = rx_ack.await.unwrap();
        new_direct.send(data).await.unwrap();

        let direct_channels = &mut self.direct_channels;
        direct_channels.insert(target_id, new_direct);
    }

    fn create_direct_communication_task(&mut self) -> mpsc::Sender<Bytes> {
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

    fn create_room_communication_task(
        &mut self,
        tx_client_room: broadcast::Sender<Bytes>,
    ) -> broadcast::Sender<Bytes> {
        let mut rx_room_comm = tx_client_room.subscribe();

        let tx_comm_client = self.comm.tx.clone();
        task::spawn(async move {
            loop {
                match rx_room_comm.recv().await {
                    Ok(data) => tx_comm_client.send(data).await.ok(),
                    Err(err) => {
                        todo!("an error occurred during room message transit: {}", err);
                    }
                };
            }
        });

        tx_client_room
    }
}
