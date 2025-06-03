use super::util::{
    config::{COMM_CLIENT_CAPACITY, DIRECT_CAPACITY, MANAGER_CLIENT_CAPACITY, ROOM_CAPACITY},
    types::{
        server_data_types::{
            BroadcastChannel, Client, ClientManagerMsg, ClientPersistenceMsg, ClientTaskResult,
            CreateRoomResponse, CreateRoomServerTransit, DirectChannelTransitPayload,
            DirectChannelTxTransit, JoinRoomServerTransit, JoinRoommPersistenceResponse,
            ManagerClientMsg, MpscChannel, MultipleRoomsUpdateTransit, RoomChannelTxTransit,
            RoomUpdateTransit, UserDataTransit,
        },
        server_error_types::{BincodeErr, TcpErr},
        server_error_wrapper_types::TcpDataParsingError,
    },
};
use crate::{
    server_lib::util::types::server_error_types::Bt,
    shared_lib::types::{
        Channel, ClientServerMsg, JoinRoomNotification, JoinRoomServerResponse, Response, RoomData,
        ServerClientMsg, User, UserInitData,
    },
};
use anyhow::{anyhow, Result};
use bytes::{Bytes, BytesMut};
use futures::{future::join_all, SinkExt, StreamExt, TryFutureExt};
use log::{debug, error, warn};
use std::collections::HashMap;
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
        user: User,
        tcp_read: &'a mut FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
        tcp_write: &'a mut FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
        tx_client_manager: mpsc::Sender<ClientManagerMsg>,
        _tx_client_persistence: mpsc::Sender<ClientPersistenceMsg>,
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

        let client_manager_channel = MpscChannel {
            tx: tx_client_manager,
            rx: rx_manager_client,
        };

        let client = Client {
            tx: tx_manager_client,
            user: User {
                username: user.username.clone(),
                id: user.id,
            },
        };

        if let Err(err) = client_manager_channel
            .tx
            .send(ClientManagerMsg::ClientConnected(client))
            .await
        {
            error!("rx_client_manager dropped {},  {}", err, Bt::new());
        };
        Self {
            username: user.username,
            id: user.id,
            direct_channels,
            room_channels,
            tcp_read,
            tcp_write,
            comm_client_data_channel,
            client_manager_channel,
            client_comm_cleanup_channel,
            comm_client_drop_channel,
            close_channel,
            tx_client_persistence: _tx_client_persistence,
        }
    }

    async fn init(&mut self) -> Result<(), TcpDataParsingError> {
        let init_data = self.get_user_init_data().await?;

        let msg = ServerClientMsg::Init(init_data.clone());
        self.send_to_client(msg).await?;

        let room_transmitters = self.get_room_transmitters(init_data.rooms.clone()).await;

        for (id, tx) in room_transmitters {
            let tx = self.spawn_room_communication_task(tx, id);

            self.room_channels.insert(id, tx);

            let msg = ServerClientMsg::UserConnected(User {
                username: self.username.clone(),
                id: self.id,
            });

            self.send_data_to_channel(msg, Channel::Room(id)).await?;
        }

        Ok(())
    }

    async fn get_user_init_data(&self) -> Result<UserInitData, anyhow::Error> {
        let (tx_ack, rx_ack) = oneshot::channel();

        let transit = UserDataTransit {
            tx: tx_ack,
            user: User {
                username: self.username.clone(),
                id: self.id,
            },
        };

        let msg = ClientPersistenceMsg::GetUserData(transit);

        self.tx_client_persistence
            .send(msg)
            .await
            .map_err(|err| anyhow!("tx_client_persistence dropped: {err} {}", Bt::new()))?;

        let init_server_data = rx_ack.await.map_err(|err| {
            anyhow!(
                "oneshot transmitter for client init got dropped: {err} {}",
                Bt::new()
            )
        })?;

        let (tx_ack, rx_ack) = oneshot::channel();

        let transit = MultipleRoomsUpdateTransit {
            tx_ack,
            rooms: init_server_data.rooms,
        };
        let msg = ClientManagerMsg::UpdateMultipleRooms(transit);

        self.client_manager_channel
            .tx
            .send(msg)
            .await
            .map_err(|err| anyhow!("{err}{}", Bt::new()))?;

        let updated_room_data = rx_ack.await.map_err(|err| anyhow!("{err}{}", Bt::new()))?;
        let init_client_data = UserInitData {
            rooms: updated_room_data,
        };

        Ok(init_client_data)
    }

    async fn get_room_transmitters(
        &self,
        rooms: Vec<RoomData>,
    ) -> Vec<(Uuid, broadcast::Sender<Bytes>)> {
        let mut handles = Vec::with_capacity(rooms.len());

        for mut room in rooms {
            let (tx_ack, rx_ack) = oneshot::channel::<broadcast::Sender<Bytes>>();
            let tx_client_manager = self.client_manager_channel.tx.clone();

            room.users.retain(|u| u.id != self.id);
            let msg = ClientManagerMsg::GetRoomChannelTx(RoomChannelTxTransit {
                room_id: room.id,
                room_users: room.users,
                ack: tx_ack,
            });

            if let Err(err) = tx_client_manager.send(msg).await {
                warn!("client_manager_channel.rx dropped, {} {}", err, Bt::new());
            };
            let handle = task::spawn(async move {
                match rx_ack.await {
                    Ok(t) => Ok((room.id, t)),
                    Err(_) => Err(format!(
                        "tokio task failed to fetch \"{}\" room transmitter {}",
                        Bt::new(),
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
                Err(err) => error!(
                    "Task handle fetching room tx failed to resolve: {err} {}",
                    Bt::new()
                ),
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

                result = self.client_manager_channel.rx.recv() =>  self.handle_manager_msg(result).await,

                result = self.comm_client_data_channel.rx.recv() => {
                    let result = match result {
                        Some(r) => r,
                        None => {
                            warn!("tx_comm_client_data dropped, Should be in comm_client_data_channel field!!!, {}",  Bt::new());
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
                            warn!("tx_comm_client_cleanup dropped. Should be in comm_client_drop_channel field!!!, {}", Bt::new());
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
            error!("rx_client_manager dropped, error: {}, {}", err, Bt::new())
        };

        for (_, tx) in &self.room_channels {
            let msg = ServerClientMsg::UserDisconnected(User {
                username: self.username.clone(),
                id: self.id,
            });
            let bytes = match bincode::serialize(&msg) {
                Ok(s) => Bytes::from(s),
                Err(err) => {
                    error!(
                        "User left notification not sent, error: {err} {}",
                        Bt::new(),
                    );
                    return;
                }
            };

            tx.send(bytes).ok();
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

    async fn send_to_client(&mut self, msg: ServerClientMsg) -> Result<(), TcpDataParsingError> {
        let serialized = bincode::serialize(&msg).map_err(|err| BincodeErr(err, Bt::new()))?;
        self.tcp_write
            .send(serialized.into())
            .map_err(|err| TcpErr(err, Bt::new()))
            .await?;
        Ok(())
    }

    async fn handle_tcp_msg(
        &mut self,
        result: Option<Result<BytesMut, std::io::Error>>,
    ) -> Result<(), TcpDataParsingError> {
        match result {
            Some(frame) => {
                let data = frame.map_err(|err| TcpErr(err, Bt::new()))?;

                let message: ClientServerMsg =
                    bincode::deserialize(&data).map_err(|err| BincodeErr(err, Bt::new()))?;

                match message {
                    ClientServerMsg::ASCII(img) => {
                        let target = img.to.clone();
                        let msg = ServerClientMsg::ASCII(img);
                        self.send_data_to_channel(msg, target).await?;
                    }
                    ClientServerMsg::Text(text_msg) => {
                        let target = text_msg.to.clone();
                        let msg = ServerClientMsg::Text(text_msg);
                        self.send_data_to_channel(msg, target).await?;
                    }
                    ClientServerMsg::FileChunk(chunk) => {
                        let target = chunk.to.clone();
                        let msg = ServerClientMsg::FileChunk(chunk);
                        self.send_data_to_channel(msg, target).await?;
                    }
                    ClientServerMsg::FileMetadata(data) => {
                        let target = data.to.clone();
                        let msg = ServerClientMsg::FileMetadata(data);
                        self.send_data_to_channel(msg, target).await?;
                    }
                    ClientServerMsg::Logout => {
                        if let Err(err) = self.close_channel.tx.send(ClientTaskResult::Logout).await
                        {
                            error!("rx close_channel dropped, {}, {}", err, Bt::new())
                        };
                    }
                    ClientServerMsg::CreateRoom(t) => {
                        let (tx_ack, rx_ack) = oneshot::channel::<CreateRoomResponse>();

                        let transit = CreateRoomServerTransit {
                            tx: tx_ack,
                            room_name: t.room_name,
                            room_pwd: t.room_password,
                            username: self.username.clone(),
                        };

                        let msg = ClientPersistenceMsg::CreateRoom(transit);

                        if let Err(err) = self.tx_client_persistence.send(msg).await {
                            error!("Persistence task not running {}, {}", err, Bt::new());
                            let res = CreateRoomResponse::Failure(String::from(
                                "Internal server error, creating room failed",
                            ));
                            let msg = ServerClientMsg::CreateRoomResponse(res);
                            self.send_to_client(msg).await?;
                            return Ok(());
                        };

                        let res = match rx_ack.await {
                            Err(err) => {
                                error!(
                                    " create room tx dropped before returning answer, err: {}, {}",
                                    err,
                                    Bt::new()
                                );
                                let res = CreateRoomResponse::Failure(String::from(
                                    "Internal server error, creating room failed",
                                ));
                                let msg = ServerClientMsg::CreateRoomResponse(res);
                                self.send_to_client(msg).await?;
                                return Ok(());
                            }
                            Ok(res) => res,
                        };

                        let msg = ServerClientMsg::CreateRoomResponse(res.clone());

                        self.send_to_client(msg).await?;

                        if let Response::Success(room) = res {
                            let (tx, _) = broadcast::channel(ROOM_CAPACITY);
                            self.room_channels.insert(room.id, tx.clone());
                            self.spawn_room_communication_task(tx, room.id);
                        };
                    }
                    ClientServerMsg::JoinRoom(t) => {
                        let (tx_ack, rx_ack) = oneshot::channel::<JoinRoommPersistenceResponse>();

                        let transit = JoinRoomServerTransit {
                            tx: tx_ack,
                            room_name: t.room_name,
                            room_pwd: t.room_password,
                            user: User {
                                id: self.id,
                                username: self.username.clone(),
                            },
                        };

                        let msg = ClientPersistenceMsg::JoinRoom(transit);

                        let server_err_res = JoinRoomServerResponse::Failure(String::from(
                            "Internal server error, joining room failed",
                        ));

                        let server_err_msg = ServerClientMsg::JoinRoomResponse(server_err_res);

                        if let Err(err) = self.tx_client_persistence.send(msg).await {
                            warn!("Persistence task not running {}, {}", err, Bt::new());
                            self.send_to_client(server_err_msg).await?;
                            return Ok(());
                        };

                        let room_data = match rx_ack.await {
                            Err(err) => {
                                error!(" oneshot tx_ack dropped in persistence_task before answering {}, {}", err, Bt::new());
                                self.send_to_client(server_err_msg).await?;
                                return Ok(());
                            }
                            Ok(res) => match res {
                                Response::Failure(reason) => {
                                    let res = Response::Failure(reason);
                                    let msg = ServerClientMsg::JoinRoomResponse(res);
                                    self.send_to_client(msg).await?;
                                    return Ok(());
                                }

                                Response::Success(mut data) => {
                                    data.users.retain(|u| u.username != self.username);
                                    data
                                }
                            },
                        };

                        let (tx_ack, rx_ack) = oneshot::channel();

                        let transit = RoomUpdateTransit {
                            tx_ack: tx_ack,
                            room: room_data,
                        };

                        let msg = ClientManagerMsg::UpdateRoom(transit);

                        if let Err(err) = self.client_manager_channel.tx.send(msg).await {
                            error!("rx_client_manager dropped, error: {}, {}", err, Bt::new())
                        };

                        let room_data = match rx_ack.await {
                            Err(err) => {
                                warn!("client_manager_channel.rx dropped, {} {}", err, Bt::new());
                                self.send_to_client(server_err_msg).await?;
                                return Ok(());
                            }
                            Ok(users) => users,
                        };

                        self.establish_room_comm(room_data.id, room_data.users.clone())
                            .await;

                        let notification = JoinRoomNotification {
                            room_id: room_data.id,
                            user: User {
                                username: self.username.clone(),
                                id: self.id,
                            },
                        };

                        let msg = ServerClientMsg::UserJoinedRoom(notification);
                        let target = Channel::Room(room_data.id);
                        self.send_data_to_channel(msg, target).await?;

                        let res = JoinRoomServerResponse::Success(room_data);
                        let msg = ServerClientMsg::JoinRoomResponse(res);
                        self.send_to_client(msg).await?;
                    }
                };
            }
            None => {
                if let Err(err) = self.close_channel.tx.send(ClientTaskResult::Close).await {
                    error!("rx close_channel dropped: {},  {}", err, Bt::new())
                };
            }
        }
        Ok(())
    }

    async fn handle_manager_msg(&mut self, result: Option<ManagerClientMsg>) {
        if let Some(msg) = result {
            match msg {
                ManagerClientMsg::EstablishDirectComm(c) => {
                    self.direct_channels
                        .insert(c.payload.from, c.payload.tx_client_client);
                    let tx_client_client = self.spawn_direct_communication_task(c.payload.from);

                    if let Err(err) = c.ack.send(tx_client_client) {
                        error!("oneshot rx not dropped during establishing direct communication. transit data: {:?}, {}", err, Bt::new());
                    };
                }

                ManagerClientMsg::GetRoomTransmitter(t) => {
                    let room_transmitter = match self.room_channels.get(&t.room_id) {
                        None => {
                            debug!(
                                "romm transmitter not found after fetching for another user! {}",
                                Bt::new()
                            );
                            return;
                        }
                        Some(tx) => tx,
                    };
                    t.tx_ack.send(room_transmitter.clone()).ok();
                }
            };
        }
    }

    async fn send_data_to_channel(
        &mut self,
        msg: ServerClientMsg,
        target: Channel,
    ) -> Result<(), TcpDataParsingError> {
        let serialized = bincode::serialize(&msg).map_err(|err| BincodeErr(err, Bt::new()))?;
        let data = Bytes::from(serialized);

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
                                    "establishing direct communication failed {}, msg not sent",
                                    Bt::new()
                                );
                                return Ok(());
                            }
                        };
                        tx
                    }
                };

                if tx.send(data).await.is_err() {
                    if self.comm_client_drop_channel.tx.send(target).await.is_err() {
                        warn!("rx comm_client_drop_channel dropped, should be saved in client_task instance!!");
                    };
                };
            }
            Channel::Room(room_id) => {
                let tx = self.room_channels.get(&room_id);

                let tx = match tx {
                    Some(tx) => tx,
                    None => {
                        warn!("Room transmiter not found {}", Bt::new());
                        return Ok(());
                    }
                };

                if let Err(err) = tx.send(data) {
                    let tx = self.spawn_room_communication_task(tx.clone(), room_id);
                    if tx.send(err.0).is_err() {
                        error!("unable to establish room communication!!{}", Bt::new());
                    };
                };
            }
        };
        Ok(())
    }

    async fn establish_direct_comm(&mut self, target_id: Uuid) {
        let tx_client_client = self.spawn_direct_communication_task(target_id);
        let (tx_ack, rx_ack) = oneshot::channel::<mpsc::Sender<Bytes>>();

        let channel_transit = DirectChannelTxTransit {
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
            .send(ClientManagerMsg::GetDirectChannelTx(channel_transit))
            .await
        {
            warn!("tx_client_manager dropped, {},{}", err, Bt::new());
            return;
        };

        let new_direct = match rx_ack.await {
            Err(err) => {
                warn!(
                    "oneshot transmitter for establishing direct communication dropped, {}, {}",
                    err,
                    Bt::new()
                );
                return;
            }
            Ok(tx) => tx,
        };

        let direct_channels = &mut self.direct_channels;
        direct_channels.insert(target_id, new_direct);
    }

    async fn establish_room_comm(&mut self, room_id: Uuid, room_users: Vec<User>) {
        let (tx_ack, rx_ack) = oneshot::channel::<broadcast::Sender<Bytes>>();

        let transit = RoomChannelTxTransit {
            room_id,
            room_users,
            ack: tx_ack,
        };

        if let Err(err) = self
            .client_manager_channel
            .tx
            .send(ClientManagerMsg::GetRoomChannelTx(transit))
            .await
        {
            warn!("tx_client_manager dropped, {},{}", err, Bt::new());
            return;
        };

        let tx = match rx_ack.await {
            Err(err) => {
                debug!(
                    "oneshot transmitter for establishing room communication dropped, {}, {}",
                    err,
                    Bt::new()
                );
                return;
            }
            Ok(tx) => tx,
        };

        let tx = self.spawn_room_communication_task(tx, room_id);
        self.room_channels.insert(room_id, tx);
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
        tx_room: broadcast::Sender<Bytes>,
        room_id: Uuid,
    ) -> broadcast::Sender<Bytes> {
        let mut rx_room_comm = tx_room.subscribe();

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
                                    warn!("room receiver not handling received messages, missed: {}, {}", n, Bt::new());

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

        tx_room
    }
}
