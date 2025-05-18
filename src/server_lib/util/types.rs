use bytes::Bytes;
use tokio::sync::{broadcast, mpsc, oneshot};
use uuid::Uuid;

use crate::shared_lib::types::{RoomChannel, User};

#[derive(Debug)]
pub struct DirectChannelTransitPayload {
    pub tx_client_client: mpsc::Sender<Bytes>,
    pub from: Uuid,
    pub to: Uuid,
}

pub struct EstablishDirectCommTransit {
    pub payload: DirectChannelTransitPayload,
    pub ack: oneshot::Sender<mpsc::Sender<Bytes>>,
}

pub struct GetAllUsersTransit {
    pub ack: oneshot::Sender<Vec<User>>,
}

pub enum ClientManagerMsg {
    ClientConnected(Client),
    ClientDropped(Uuid),
    EstablishDirectComm(EstablishDirectCommTransit),
    EstablishRoomComm(EstablishRoomCommTransit),
    CheckUsername(CheckUsernameTransit),
    GetOnlineUsers(GetAllUsersTransit),
}

pub struct EstablishRoomCommTransit {
    pub room_id: Uuid,
    pub room_users: Vec<User>,
    pub ack: oneshot::Sender<broadcast::Sender<Bytes>>,
}

pub struct CheckUsernameTransit {
    pub username: String,
    pub tx: oneshot::Sender<bool>,
}

pub enum ManagerClientMsg {
    EstablishDirectComm(EstablishDirectCommTransit),
    GetRoomTransmitter(GetRoomTransmitterTransit),
}

pub struct GetRoomTransmitterTransit {
    pub tx_ack: oneshot::Sender<broadcast::Sender<Bytes>>,
    pub room_id: Uuid,
}

pub struct JoinRoomTransit {
    pub room_id: Uuid,
    pub tx: oneshot::Sender<broadcast::Sender<Bytes>>,
}

pub struct MpscChannel<T = Bytes, R = Bytes> {
    pub tx: mpsc::Sender<T>,
    pub rx: mpsc::Receiver<R>,
}
pub struct BroadcastChannel<T = Bytes, R = Bytes> {
    pub tx: broadcast::Sender<T>,
    pub rx: broadcast::Receiver<R>,
}

pub struct OneShotChannel<T> {
    pub tx: oneshot::Sender<T>,
    pub rx: oneshot::Receiver<T>,
}

pub struct Client {
    pub user: User,
    pub tx: mpsc::Sender<ManagerClientMsg>,
}

pub enum ClientTaskResult {
    Close,
    Logout,
}

#[derive(Debug)]
pub enum ClientPersistenceMsg {
    GetUserData(GetUserDataTransit),
    UserJoinedRoom(ServerRoomUpdateTransit),
    UserLeftRoom(ServerRoomUpdateTransit),
}

#[derive(Debug)]
pub struct ServerRoomUpdateTransit {
    pub user: User,
    pub room_id: Uuid,
}

#[derive(Debug)]
pub struct GetUserDataTransit {
    pub tx: oneshot::Sender<PersistedUserData>,
    pub user: User,
}

#[derive(Debug)]
pub struct PersistedUserData {
    pub rooms: Vec<RoomChannel>,
}
