use bytes::Bytes;
use tokio::sync::{broadcast, mpsc, oneshot};
use uuid::Uuid;

use crate::shared_lib::types::{
    AuthData, AuthResponse, RegisterData, RegisterResponse, RoomChannel, TuiRoom, User,
};

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

pub struct GetConnectedUsersTransit {
    pub tx_ack: oneshot::Sender<Vec<TuiRoom>>,
    pub rooms: Vec<RoomChannel>,
}

pub enum ClientManagerMsg {
    ClientConnected(Client),
    ClientDropped(Uuid),
    EstablishDirectComm(EstablishDirectCommTransit),
    EstablishRoomComm(EstablishRoomCommTransit),
    GetConnectedUsers(GetConnectedUsersTransit),
    UserRegistered(User),
    IsOnline(IsOnlineTransit),
}

pub struct IsOnlineTransit {
    pub ack: oneshot::Sender<bool>,
    pub username: String,
}

pub struct EstablishRoomCommTransit {
    pub room_id: Uuid,
    pub room_users: Vec<User>,
    pub ack: oneshot::Sender<broadcast::Sender<Bytes>>,
}

pub enum ManagerClientMsg {
    EstablishDirectComm(EstablishDirectCommTransit),
    GetRoomTransmitter(GetRoomTxTransit),
}

pub struct GetRoomTxTransit {
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

#[derive(Debug)]
pub struct Client {
    pub user: User,
    pub tx: mpsc::Sender<ManagerClientMsg>,
}

pub enum ClientTaskResult {
    Close,
    Logout,
}

pub enum ClientPersistenceMsg {
    GetUserData(UserDataTransit),
    UserJoinedRoom(UserRoomTransit),
    UserLeftRoom(UserRoomTransit),
    Register(RegisterDataTransit),
    Authenticate(AuthTransit),
}

pub struct AuthTransit {
    pub tx: oneshot::Sender<AuthResponse>,
    pub data: AuthData,
}

pub struct RegisterDataTransit {
    pub data: RegisterData,
    pub tx: oneshot::Sender<RegisterResponse>,
}

#[derive(Debug)]
pub struct UserRoomTransit {
    pub user: User,
    pub room_id: Uuid,
}

#[derive(Debug)]
pub struct UserDataTransit {
    pub tx: oneshot::Sender<UserServerData>,
    pub user: User,
}
#[derive(Debug)]
pub struct UserServerData {
    pub rooms: Vec<RoomChannel>,
}

pub struct DbUser {
    pub username: String,
    pub id: Uuid,
    pub password: String,
    pub rooms: Vec<Uuid>,
}

pub struct DbRoomChannel {
    pub users: Vec<String>,
    pub id: Uuid,
    pub password: Option<String>,
}
