use super::util::{
    config::{COMM_CLIENT_CAPACITY, DIRECT_CAPACITY, MANAGER_CLIENT_CAPACITY},
    errors::{ClientInitError, DataParsingError},
    server_functions::{serialize_file_chunk, serialize_file_metadata, serialize_text_msg},
    types::{
        BroadcastChannel, Client, ClientManagerMsg, ClientPersistenceMsg, ClientTaskResult,
        DirectChannelTransitPayload, EstablishDirectCommTransit, EstablishRoomCommTransit,
        GetUserDataTransit, ManagerClientMsg, MpscChannel, ServerRoomUpdateTransit,
    },
};
use crate::{
    server_lib::util::server_functions::get_location,
    shared_lib::{
        config::PUBLIC_ROOM_ID,
        types::{
            Channel, ClientRoomUpdateTransit, ClientServerMsg, InitPersistedUserData, InitUserData,
            RoomChannel, ServerClientMsg, User,
        },
    },
};
use anyhow::Result;
use bytes::{Bytes, BytesMut};
use futures::{future::join_all, SinkExt, StreamExt};
use log::{debug, error, info, warn};
use std::{collections::HashMap, str::FromStr};
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

pub struct ClientTask<'a> {
    username: String,
    id: Uuid,
    client_manager_channel: MpscChannel<ClientManagerMsg, ManagerClientMsg>,
    comm_client_data_channel: MpscChannel,
    comm_client_drop_channel: MpscChannel<Channel, Channel>,
    client_comm_cleanup_channel: BroadcastChannel<(), ()>,
    close_channel: MpscChannel<ClientTaskResult, ClientTaskResult>,
    tcp_read: &'a mut FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
    tcp_write: &'a mut FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
    room_channels: HashMap<Uuid, broadcast::Sender<Bytes>>,
    direct_channels: HashMap<Uuid, mpsc::Sender<Bytes>>,
    tx_client_persistence: mpsc::Sender<ClientPersistenceMsg>,
}

impl<'a> ClientTask<'a> {
    pub async fn new(
        init_data: InitUserData,
        tcp_read: &'a mut FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
        tcp_write: &'a mut FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
        tx_client_manager: mpsc::Sender<ClientManagerMsg>,
        tx_client_persistence: mpsc::Sender<ClientPersistenceMsg>,
    ) -> Self {
        let room_channels = HashMap::new();
        let direct_channels = HashMap::new();

        let (tx_manager_client, rx_manager_client) =
            mpsc::channel::<ManagerClientMsg>(MANAGER_CLIENT_CAPACITY);

        let (tx_cleanup, rx_cleanup) = broadcast::channel::<()>(1);
        let client_comm_cleanup_channel = BroadcastChannel {
            tx: tx_cleanup,
            rx: rx_cleanup,
        };

        let (tx_comm_client, rx_comm_client) = mpsc::channel::<Bytes>(COMM_CLIENT_CAPACITY);
        let comm_client_data_channel = MpscChannel {
            tx: tx_comm_client,
            rx: rx_comm_client,
        };

        let (tx_comm_drop, rx_comm_drop) = mpsc::channel::<Channel>(COMM_CLIENT_CAPACITY);
        let comm_client_drop_channel = MpscChannel::<Channel, Channel> {
            tx: tx_comm_drop,
            rx: rx_comm_drop,
        };

        let (tx_close, rx_close) = mpsc::channel::<ClientTaskResult>(COMM_CLIENT_CAPACITY);
        let close_channel = MpscChannel {
            tx: tx_close,
            rx: rx_close,
        };

        let client = Client {
            tx: tx_manager_client,
            user: User {
                username: init_data.username.clone(),
                id: init_data.id,
            },
        };

        // let public_room_details = ServerRoomUpdateTransit {
        //     room_id: Uuid::from_str(PUBLIC_ROOM_ID).unwrap(),
        //     user: User {
        //         username: init_data.username.clone(),
        //         id: init_data.id,
        //     },
        // };

        // if let Err(err) = tx_client_persistence
        //     .send(ClientPersistenceMsg::UserJoinedRoom(public_room_details))
        //     .await
        // {
        //     error!("{},  {}", err, get_location());
        // }

        if let Err(err) = tx_client_manager
            .send(ClientManagerMsg::ClientConnected(client))
            .await
        {
            error!("{},  {}", err, get_location());
        };

        let client_manager_channel = MpscChannel {
            tx: tx_client_manager,
            rx: rx_manager_client,
        };

        Self {
            username: init_data.username,
            id: init_data.id,
            direct_channels,
            room_channels,
            tcp_read,
            tcp_write,
            comm_client_data_channel,
            client_manager_channel,
            client_comm_cleanup_channel,
            comm_client_drop_channel,
            close_channel,
            tx_client_persistence,
        }
    }

    async fn init(&mut self) -> Result<(), ClientInitError> {
        let (tx_ack, rx_ack) = oneshot::channel();

        let msg = ClientPersistenceMsg::GetUserData(GetUserDataTransit {
            tx: tx_ack,
            user: User {
                username: self.username.clone(),
                id: self.id,
            },
        });

        self.tx_client_persistence.send(msg).await.map_err(|err| {
            ClientInitError::Unexpected(format!(
                "tx_client_persistence   dropped: {err} {}",
                get_location()
            ))
        })?;

        let persited_data = rx_ack.await.map_err(|err| {
            ClientInitError::Unexpected(format!(
                "oneshot transmitter for client init got dropped: {err} {}",
                get_location()
            ))
        })?;

        let room_transmitters = self
            .get_room_transmitters(persited_data.rooms.clone())
            .await;

        for (id, tx) in room_transmitters {
            let transit = ClientRoomUpdateTransit {
                room_id: id,
                user: User {
                    username: self.username.clone(),
                    id: self.id,
                },
            };

            let msg = ServerClientMsg::UserJoinedRoom(transit);
            let serialized =
                bincode::serialize(&msg).map_err(|err| DataParsingError::Bincode(err))?;

            let bytes = Bytes::from(serialized);
            tx.send(bytes).ok();

            let tx = self.spawn_room_communication_task(tx, id);
            self.room_channels.insert(id, tx);
        }

        let init_data = InitPersistedUserData {
            rooms: persited_data.rooms,
        };

        let msg = ServerClientMsg::Init(init_data);

        let serialized = bincode::serialize(&msg).map_err(|err| DataParsingError::Bincode(err))?;

        self.tcp_write
            .send(serialized.into())
            .await
            .map_err(|err| DataParsingError::TcpReadWrite(err))?;

        let public_room_details = ServerRoomUpdateTransit {
            room_id: Uuid::from_str(PUBLIC_ROOM_ID).unwrap(),
            user: User {
                username: self.username.clone(),
                id: self.id,
            },
        };

        if let Err(err) = self
            .tx_client_persistence
            .send(ClientPersistenceMsg::UserJoinedRoom(public_room_details))
            .await
        {
            error!("{},  {}", err, get_location());
        }

        Ok(())
    }

    async fn get_room_transmitters(
        &self,
        rooms: Vec<RoomChannel>,
    ) -> Vec<(Uuid, broadcast::Sender<Bytes>)> {
        let mut handles = Vec::with_capacity(rooms.len());

        for room in rooms {
            let (tx_ack, rx_ack) = oneshot::channel::<broadcast::Sender<Bytes>>();
            let tx_client_manager = self.client_manager_channel.tx.clone();

            let msg = ClientManagerMsg::EstablishRoomComm(EstablishRoomCommTransit {
                room_id: room.id,
                room_users: room.users,
                ack: tx_ack,
            });

            let handle = task::spawn(async move {
                if let Err(err) = tx_client_manager.send(msg).await {
                    warn!(
                        "client_manager_channel.rx dropped, {} {}",
                        err,
                        get_location()
                    );
                };
                match rx_ack.await {
                    Ok(t) => Ok((room.id, t)),
                    Err(_) => Err(format!(
                        "tokio task failed to fetch \"{}\" room transmitter ",
                        { room.name }
                    )),
                }
            });

            handles.push(handle);
        }

        let results = join_all(handles).await;
        let mut room_channels = Vec::new();

        for res in results {
            match res {
                Ok(val) => match val {
                    Ok(t) => room_channels.push(t),
                    Err(err_msg) => error!("{err_msg}"),
                },
                Err(err) => error!("fetching of room broadcast transmitter failed: {}", err),
            }
        }

        room_channels
    }

    pub async fn run(mut self) -> ClientTaskResult {
        if let Err(err) = self.init().await {
            error!("{err}");
            self.cleanup().await;
            return ClientTaskResult::Close;
        };

        let result = loop {
            select! {
                result = self.tcp_read.next() => if let Err(err) = self.handle_tcp_msg(result).await  {
                    error!("data processing error: {}", err);
                    break ClientTaskResult::Close;
                },

                result = self.client_manager_channel.rx.recv() => if let Err(err) = self.handle_manager_msg(result).await{
                    warn!("tx_manager_client dropped : {}", err);
                    break ClientTaskResult::Close
                },

                result = self.comm_client_data_channel.rx.recv() => {
                    let result = match result {
                        Some(r) => r,
                        None => {
                            warn!("tx_comm_client_data dropped, Should be in comm_client_data_channel field!!!, {}",  get_location());
                            break ClientTaskResult::Close;
                        }
                    };

                    if let Err(err) = self.tcp_write.send(result).await{
                        error!("Error writing data to TCP, :{}",err);
                        break ClientTaskResult::Close;
                    };
                }

                result = self.comm_client_drop_channel.rx.recv() => {
                    let result = match result {
                        Some(r) => r,
                        None => {
                            warn!("tx_comm_client_cleanup dropped. Should be in comm_client_drop_channel field!!!, {}", get_location());
                            break ClientTaskResult::Close;
                        }
                    };
                    self.handle_comm_drop(result);
                }

                result = self.close_channel.rx.recv() => {
                    if let Some(res) = result {
                        break res;
                    };
                }
            }
        };

        self.cleanup().await;
        result
    }

    async fn cleanup(&mut self) {
        self.client_comm_cleanup_channel.tx.send(()).ok();

        let msg = ClientManagerMsg::ClientDropped(self.id);
        if let Err(err) = self.client_manager_channel.tx.send(msg).await {
            error!(
                "rx_client_manager dropped, error: {}, {}",
                err,
                get_location()
            )
        };

        for (id, tx) in &self.room_channels {
            let server_update = ServerRoomUpdateTransit {
                room_id: *id,
                user: User {
                    username: self.username.clone(),
                    id: self.id,
                },
            };

            let msg = ClientPersistenceMsg::UserLeftRoom(server_update);
            if let Err(err) = self.tx_client_persistence.send(msg).await {
                error!(
                    "tx_client_persistence dropped, error: {}, {}",
                    err,
                    get_location()
                )
            };

            let tui_update = ClientRoomUpdateTransit {
                room_id: *id,
                user: User {
                    username: self.username.clone(),
                    id: self.id,
                },
            };

            let msg = ServerClientMsg::UserLeftRoom(tui_update);
            let bytes = match bincode::serialize(&msg) {
                Ok(s) => Bytes::from(s),
                Err(err) => {
                    error!(
                        "Bincode error, user left notification not sent, error: {} {}",
                        err,
                        get_location(),
                    );
                    return;
                }
            };

            if let Err(err) = tx.send(bytes) {
                error!(
                    "Channel transmitter dropped preemptively, {}, \"user left\" notification nots sent  {}",
                    err,
                    get_location()
                );
            };
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
                let data = frame.map_err(|err| DataParsingError::from(err))?;

                let message: ClientServerMsg =
                    bincode::deserialize(&data).map_err(|err| DataParsingError::from(err))?;

                match message {
                    ClientServerMsg::Text(msg) => {
                        let data = serialize_text_msg(msg.clone())?;
                        self.send_data(data, msg.to).await
                    }
                    ClientServerMsg::FileChunk(c) => {
                        let data = serialize_file_chunk(c.clone())?;
                        self.send_data(data, c.to).await
                    }
                    ClientServerMsg::FileMetadata(m) => {
                        let data = serialize_file_metadata(m.clone())?;
                        self.send_data(data, m.to).await
                    }
                    ClientServerMsg::Logout => {
                        if let Err(err) = self.close_channel.tx.send(ClientTaskResult::Logout).await
                        {
                            error!("rx close_channel dropped, {}, {}", err, get_location())
                        };
                    }
                };
            }
            None => {
                if let Err(err) = self.close_channel.tx.send(ClientTaskResult::Close).await {
                    error!("rx close_channel dropped: {},  {}", err, get_location())
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

                    let tx_client_client = self.spawn_direct_communication_task(c.payload.from);

                    if let Err(err) = c.ack.send(tx_client_client) {
                        error!("oneshot rx not dropped during establishing direct communication. transit data: {:?}, {}", err, get_location());
                    };
                }

                ManagerClientMsg::GetRoomTransmitter(t) => {
                    let room_transmitter = match self.room_channels.get(&t.room_id) {
                        None => {
                            warn!(
                                "romm transmitter not found after fetching for another user! {}",
                                get_location()
                            );
                            return Ok(());
                        }
                        Some(tx) => tx,
                    };
                    t.tx_ack.send(room_transmitter.clone()).ok();
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
                        self.establish_direct_comm(target_id).await;
                        let tx = match self.direct_channels.get(&target_id) {
                            Some(tx) => tx,
                            None => {
                                warn!(
                                    "establishing direct communication failed tx not found,  {}",
                                    get_location()
                                );
                                return;
                            }
                        };
                        tx
                    }
                };

                if tx.send(data).await.is_err() {
                    warn!("should be already removed!! {}", get_location());
                    self.comm_client_drop_channel.tx.send(target).await.ok();
                };
            }
            Channel::Room(target_id) => {
                let tx = self.room_channels.get(&target_id);

                let tx = match tx {
                    Some(tx) => tx,
                    None => {
                        todo!("room not found, implement err handaling");
                    }
                };
                if let Err(err) = tx.send(data) {
                    warn!("error sending data: {}, {}", err, get_location());
                    self.room_channels.remove(&target_id);
                };
            }
        };
    }

    async fn establish_direct_comm(&mut self, target_id: Uuid) {
        let tx_client_client = self.spawn_direct_communication_task(target_id);
        let (tx_ack, rx_ack) = oneshot::channel::<mpsc::Sender<Bytes>>();

        let channel_transit = EstablishDirectCommTransit {
            payload: DirectChannelTransitPayload {
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
            warn!("tx_client_manager dropped, {},{}", err, get_location());
            return;
        };

        let new_direct = match rx_ack.await {
            Err(err) => {
                warn!(
                    "oneshot transmitter for establishing direct communication dropped, {}, {}",
                    err,
                    get_location()
                );
                return;
            }
            Ok(tx) => tx,
        };

        let direct_channels = &mut self.direct_channels;
        direct_channels.insert(target_id, new_direct);
    }

    fn spawn_direct_communication_task(&mut self, direct_channel_id: Uuid) -> mpsc::Sender<Bytes> {
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
                             break;
                            }
                        },
                        _ = rx_cleanup.recv() => {
                            break;
                        },
                    };
            }
            debug!("direct communication task dropping");
        });

        tx_client_client
    }

    fn spawn_room_communication_task(
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
                                    break
                                },
                                RecvError::Lagged(n) =>{
                                    warn!("room receiver not handling received messages, missed: {}, {}", n, get_location());

                                }
                            };
                        }
                    },

                    _ = rx_cleanup.recv() => {
                        break;
                    }
                };
            }
            debug!("room communication task dropping");
        });

        tx_client_room
    }
}
