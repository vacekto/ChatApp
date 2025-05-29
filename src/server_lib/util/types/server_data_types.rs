use std::collections::VecDeque;

use bytes::Bytes;
use tokio::sync::{broadcast, mpsc, oneshot};
use uuid::Uuid;

use crate::shared_lib::types::{
    AuthData, AuthResponse, RegisterData, RegisterResponse, Response, TuiMsg, TuiRoom, User,
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
    pub rooms: Vec<DbRoom>,
}

pub enum ClientManagerMsg {
    ClientConnected(Client),
    ClientDropped(Uuid),
    EstablishDirectComm(EstablishDirectCommTransit),
    EstablishRoomComm(EstablishRoomCommTransit),
    GetOnlineUsers(GetConnectedUsersTransit),
    UserRegistered(User),
    IsOnline(IsOnlineTransit),
    GetRoomOnlineUsers(OnlineRoomUsersTransit),
}

pub struct OnlineRoomUsersTransit {
    pub tx_acks: oneshot::Sender<Vec<User>>,
    pub users: Vec<User>,
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
    UserJoinedRoom(UserRoomData),
    UserLeftRoom(UserRoomData),
    Register(RegisterDataTransit),
    Authenticate(AuthTransit),
    CreateRoom(CreateRoomServerTransit),
    JoinRoom(JoinRoomServerTransit),
}

pub type CreateRoomResponse = Response<TuiRoom>;
pub type JoinRoommPersistenceResponse = Response<JoinRoomPersistenceData>;
pub struct JoinRoomPersistenceData {
    pub room_users: Vec<User>,
    pub room_id: Uuid,
    pub room_name: String,
}

pub struct JoinRoomServerTransit {
    pub tx: oneshot::Sender<JoinRoommPersistenceResponse>,
    pub room_name: String,
    pub room_password: Option<String>,
    pub user: User,
}

pub struct CreateRoomServerTransit {
    pub tx: oneshot::Sender<CreateRoomResponse>,
    pub room_name: String,
    pub room_password: Option<String>,
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct DbRoom {
    pub id: Uuid,
    pub name: String,
    pub messages: VecDeque<TuiMsg>,
    pub users: Vec<User>,
    pub password: Option<String>,
}

pub struct AuthTransit {
    pub tx: oneshot::Sender<AuthResponse>,
    pub data: AuthData,
}

pub struct RegisterDataTransit {
    pub tx: oneshot::Sender<RegisterResponse>,
    pub data: RegisterData,
}

#[derive(Debug)]
pub struct UserRoomData {
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
    pub rooms: Vec<DbRoom>,
}

pub struct DbUser {
    pub username: String,
    pub id: Uuid,
    pub password: String,
    pub rooms: Vec<Uuid>,
}
