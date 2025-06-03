use bytes::Bytes;
use mongodb::bson::Bson;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, oneshot};
use uuid::Uuid;

use crate::shared_lib::types::{
    AuthData, AuthResponse, RegisterData, RegisterResponse, Response, RoomData, User, UserInitData,
};

#[derive(Debug)]
pub struct DirectChannelTransitPayload {
    pub tx_client_client: mpsc::Sender<Bytes>,
    pub from: Uuid,
    pub to: Uuid,
}

#[derive(Debug)]
pub struct DirectChannelTxTransit {
    pub payload: DirectChannelTransitPayload,
    pub ack: oneshot::Sender<mpsc::Sender<Bytes>>,
}

#[derive(Debug)]
pub enum ClientManagerMsg {
    ClientConnected(Client),
    ClientDropped(Uuid),
    GetDirectChannelTx(DirectChannelTxTransit),
    GetRoomChannelTx(RoomChannelTxTransit),
    UpdateRoom(RoomUpdateTransit),
    UpdateMultipleRooms(MultipleRoomsUpdateTransit),
    UserRegistered(User),
    IsOnline(IsOnlineTransit),
}

#[derive(Debug)]
pub struct RoomUpdateTransit {
    pub tx_ack: oneshot::Sender<RoomData>,
    pub room: RoomData,
}

#[derive(Debug)]
pub struct MultipleRoomsUpdateTransit {
    pub tx_ack: oneshot::Sender<Vec<RoomData>>,
    pub rooms: Vec<RoomData>,
}

#[derive(Debug)]
pub struct IsOnlineTransit {
    pub ack: oneshot::Sender<bool>,
    pub username: String,
}

#[derive(Debug)]
pub struct RoomChannelTxTransit {
    pub room_id: Uuid,
    pub room_users: Vec<User>,
    pub ack: oneshot::Sender<broadcast::Sender<Bytes>>,
}

pub enum ManagerClientMsg {
    EstablishDirectComm(DirectChannelTxTransit),
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

#[derive(Debug)]
pub enum ClientPersistenceMsg {
    GetUserData(UserDataTransit),
    UserJoinedRoom(UserRoomData),
    UserLeftRoom(UserRoomData),
    Register(RegisterDataTransit),
    Authenticate(AuthTransit),
    CreateRoom(CreateRoomServerTransit),
    JoinRoom(JoinRoomServerTransit),
}

pub type CreateRoomResponse = Response<RoomData>;
pub type JoinRoommPersistenceResponse = Response<RoomData>;

#[derive(Debug)]
pub struct JoinRoomServerTransit {
    pub tx: oneshot::Sender<JoinRoommPersistenceResponse>,
    pub room_name: String,
    pub room_pwd: Option<String>,
    pub user: User,
}

#[derive(Debug)]
pub struct CreateRoomServerTransit {
    pub tx: oneshot::Sender<CreateRoomResponse>,
    pub room_name: String,
    pub room_pwd: Option<String>,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbRoom {
    pub id: Bson,
    pub name: String,
    pub user_ids: Vec<Bson>,
    pub pwd: Option<String>,
}

#[derive(Debug)]
pub struct AuthTransit {
    pub tx: oneshot::Sender<AuthResponse>,
    pub data: AuthData,
}

#[derive(Debug)]
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
    pub tx: oneshot::Sender<UserInitData>,
    pub user: User,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DbUser {
    pub username: String,
    pub id: Bson,
    pub pwd: String,
    pub room_ids: Vec<Bson>,
}
