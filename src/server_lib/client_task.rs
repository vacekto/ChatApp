use std::collections::HashMap;

use anyhow::anyhow;
use bytes::{Bytes, BytesMut};
use futures::{SinkExt, StreamExt};
use tokio::{
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
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
    shared_lib::types::{Channel, InitClientData, ServerTuiMsg, TuiServerMsg, User},
};

use super::util::{
    config::{COMM_CLIENT_CAPACITY, DIRECT_CAPACITY, MANAGER_CLIENT_CAPACITY},
    errors::DataParsingError,
    server_functions::{serialize_file_chunk, serialize_file_metadata, serialize_text_msg},
    types::{
        BroadcastChannel, ChannelTransitPayload, Client, ClientManagerMsg, DirectChannelTransit,
        ManagerClientMsg, MpscChannel,
    },
};

pub struct ClientTask {
    _username: String,
    id: Uuid,
    client_manager_channel: MpscChannel<ClientManagerMsg, ManagerClientMsg>,
    comm_client_data_channel: MpscChannel,
    comm_client_drop_channel: MpscChannel<Channel, Channel>,
    client_comm_cleanup_channel: BroadcastChannel<(), ()>,
    close_channel: MpscChannel<(), ()>,
    tcp_read: FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
    tcp_write: FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
    room_channels: HashMap<Uuid, broadcast::Sender<Bytes>>,
    direct_channels: HashMap<Uuid, mpsc::Sender<Bytes>>,
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

        let (tx_cleanup, rx_cleanup) = broadcast::channel::<()>(1);
        let cleanup_channel = BroadcastChannel {
            tx: tx_cleanup,
            rx: rx_cleanup,
        };

        let (tx_comm_client, rx_comm_client) = mpsc::channel::<Bytes>(COMM_CLIENT_CAPACITY);
        let comm_client_channel = MpscChannel {
            tx: tx_comm_client,
            rx: rx_comm_client,
        };

        let (tx_comm_drop, rx_comm_drop) = mpsc::channel::<Channel>(COMM_CLIENT_CAPACITY);
        let comm_drop_channel = MpscChannel::<Channel, Channel> {
            tx: tx_comm_drop,
            rx: rx_comm_drop,
        };

        let client = Client {
            tx: tx_manager_client,
            user: User {
                username: init_data.username.clone(),
                id,
            },
        };

        let (tx_close, rx_close) = mpsc::channel::<()>(COMM_CLIENT_CAPACITY);
        let close_channel = MpscChannel {
            tx: tx_close,
            rx: rx_close,
        };

        if let Err(err) = tx_client_manager.send(ClientManagerMsg::Init(client)).await {
            log(err.into(), Some("client_manager not listening"));
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
            comm_client_data_channel: comm_client_channel,
            client_manager_channel: manager,
            client_comm_cleanup_channel: cleanup_channel,
            comm_client_drop_channel: comm_drop_channel,
            close_channel,
        }
    }

    pub async fn run(mut self) {
        loop {
            select! {
                result = self.tcp_read.next() => if let Err(err) = self.handle_tcp_msg(result).await {
                    log(err.into(), Some("Error reading data to client in tcp stream instance."));
                    return;
                },

                result = self.client_manager_channel.rx.recv() => if let Err(err) = self.handle_manager_msg(result).await{
                    log(err.into(), Some("Transmitter from manager client dropped."));
                    return
                },

                result = self.comm_client_data_channel.rx.recv() => {
                    let result = match result {
                        Some(r) => r,
                        None => {
                            log(anyhow!("Transmitter of communication task to client task for bytes of data dropped. Should be in comm_client_data_channel field!!!"), None);
                            return;
                        }
                    };

                    if let Err(err) = self.tcp_write.send(result).await{
                        log(err.into(), Some("Error writing data to client in tcp stream instance."));
                        return;
                    };
                }

                result = self.comm_client_drop_channel.rx.recv() => {
                    let result = match result {
                        Some(r) => r,
                        None => {
                            log(anyhow!("Transmitter of communication task to client task for channel drop notification dropped. Should be in comm_client_drop_channel field!!!"), None);
                            return;
                        }
                    };
                    self.handle_comm_drop(result);

                }

                _ = self.close_channel.rx.recv() => {
                    self.client_comm_cleanup_channel.tx.send(()).ok();
                    return;
                }
            }
        }
    }

    fn handle_comm_drop(&mut self, channel: Channel) {
        match channel {
            Channel::Room(id) => {
                self.room_channels.remove(&id);
            }
            Channel::User(id) => {
                self.direct_channels.remove(&id);
            }
        };
    }

    async fn handle_tcp_msg(
        &mut self,
        result: Option<Result<BytesMut, std::io::Error>>,
    ) -> Result<(), DataParsingError> {
        match result {
            Some(frame) => {
                let data: Bytes = frame.map_err(|err| DataParsingError::from(err))?.into();

                let message: TuiServerMsg =
                    bincode::deserialize(&data).map_err(|err| DataParsingError::from(err))?;

                match message {
                    TuiServerMsg::Text(msg) => {
                        let data = serialize_text_msg(msg.clone())?;
                        self.send_data(data, msg.to).await
                    }
                    TuiServerMsg::FileChunk(c) => {
                        let data = serialize_file_chunk(c.clone())?;
                        self.send_data(data, c.to).await
                    }
                    TuiServerMsg::FileMetadata(m) => {
                        let data = serialize_file_metadata(m.clone())?;
                        self.send_data(data, m.to).await
                    }
                };
            }
            None => {
                let msg = ClientManagerMsg::ClientDropped(self.id);
                if let Err(err) = self.client_manager_channel.tx.send(msg).await {
                    log(err.into(), Some("rx_client_manager dropped"))
                };
                if let Err(err) = self.close_channel.tx.send(()).await {
                    log(err.into(), Some("rx close_channel dropped"))
                };
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

                    let tx_client_client = self.create_direct_communication_task(c.payload.from);
                    if c.ack.send(tx_client_client).is_err() {
                        log(anyhow!("establishing direct communication"), None);
                    };
                }
                ManagerClientMsg::JoinRoom(transit) => {
                    let room_id = transit.room.id;
                    self.room_channels.insert(room_id, transit.tx.clone());

                    let msg = ServerTuiMsg::JoinRoom(transit.room.clone());
                    let bytes = bincode::serialize(&msg)?;
                    self.tcp_write.send(bytes.into()).await?;

                    let tx = self.create_room_communication_task(transit.tx, room_id);

                    let msg = ServerTuiMsg::RoomUpdate(transit.room);

                    let bytes = bincode::serialize(&msg)?.into();
                    if let Err(err) = tx.send(bytes) {
                        log(err.into(), Some("joining room"));
                    };
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

                if let Err(err) = tx.send(data).await {
                    log(err.into(), Some("sending data"));
                };
            }
            Channel::Room(target_id) => {
                let tx = self
                    .room_channels
                    .get(&target_id)
                    .expect("room not found, implement err handaling");
                if let Err(err) = tx.send(data) {
                    log(err.into(), Some("sending data"));
                };
            }
        };
    }

    async fn establish_direct_comm(&mut self, target_id: Uuid, data: Bytes) {
        let (tx_ack, rx_ack) = oneshot::channel::<mpsc::Sender<Bytes>>();
        let tx_client_client = self.create_direct_communication_task(target_id);

        let channel_transit = DirectChannelTransit {
            payload: ChannelTransitPayload {
                tx_client_client,
                from: self.id,
                to: target_id,
            },
            ack: tx_ack,
        };

        if let Err(err) = self
            .client_manager_channel
            .tx
            .send(ClientManagerMsg::EstablishDirectComm(channel_transit))
            .await
        {
            log(err.into(), None);
            return;
        };

        let new_direct = match rx_ack.await {
            Err(err) => {
                log(err.into(), Some("establishing direct communication"));
                return;
            }
            Ok(tx) => tx,
        };
        if let Err(err) = new_direct.send(data).await {
            log(err.into(), Some("establishing direct communication"));
        };

        let direct_channels = &mut self.direct_channels;
        direct_channels.insert(target_id, new_direct);
    }

    fn create_direct_communication_task(&mut self, direct_channel_id: Uuid) -> mpsc::Sender<Bytes> {
        let (tx_client_client, mut rx_client_client) = mpsc::channel::<Bytes>(DIRECT_CAPACITY);

        let mut rx_cleanup = self.client_comm_cleanup_channel.tx.subscribe();
        let tx_comm_client_data = self.comm_client_data_channel.tx.clone();
        let tx_comm_client_drop = self.comm_client_drop_channel.tx.clone();

        task::spawn(async move {
            loop {
                select! {
                    result = rx_client_client.recv() => match result {
                            Some(data) => {
                                tx_comm_client_data.send(data).await.ok();
                            },
                            None => {
                                 tx_comm_client_drop.send(Channel::User(direct_channel_id)).await.ok();
                            }
                    },
                    _ = rx_cleanup.recv() => {
                        break;
                    },
                };
            }
        });

        tx_client_client
    }

    fn create_room_communication_task(
        &mut self,
        tx_client_room: broadcast::Sender<Bytes>,
        room_id: Uuid,
    ) -> broadcast::Sender<Bytes> {
        let mut rx_room_comm = tx_client_room.subscribe();

        let mut rx_cleanup = self.client_comm_cleanup_channel.tx.subscribe();
        let tx_comm_client_data = self.comm_client_data_channel.tx.clone();
        let tx_comm_client_drop = self.comm_client_drop_channel.tx.clone();

        task::spawn(async move {
            loop {
                select! {
                    result = rx_room_comm.recv() => match result {
                        Ok(data) => {
                            tx_comm_client_data.send(data).await.ok();
                        },
                        Err(err) => {
                            match err {
                                RecvError::Closed => {
                                    tx_comm_client_drop.send(Channel::Room(room_id)).await.ok();
                                },
                                RecvError::Lagged(n) =>{
                                    log(err.into(), Some(&format!("room receiver not handling received messages, missed: {}", n)));

                                }
                            };
                        }
                    },

                    _ = rx_cleanup.recv() => {
                        break;
                    }
                };
            }
        });

        tx_client_room
    }
}
