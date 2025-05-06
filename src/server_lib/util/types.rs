use bytes::Bytes;
use tokio::sync::{broadcast, mpsc, oneshot};
use uuid::Uuid;

use crate::shared_lib::types::{RoomChannel, User};

#[derive(Debug)]
pub struct ChannelTransitPayload {
    pub tx_client_client: mpsc::Sender<Bytes>,
    pub from: Uuid,
    pub to: Uuid,
}

pub struct DirectChannelTransit {
    pub payload: ChannelTransitPayload,
    pub ack: oneshot::Sender<mpsc::Sender<Bytes>>,
}

pub enum ClientManagerMsg {
    Init(Client),
    ClientDropped(Uuid),
    EstablishDirectComm(DirectChannelTransit),
    CheckUsername(UsernameCheck),
}

pub struct UsernameCheck {
    pub username: String,
    pub tx: oneshot::Sender<bool>,
}

pub enum ManagerClientMsg {
    EstablishDirectComm(DirectChannelTransit),
    JoinRoom(RoomChannelTransit),
}

pub struct RoomChannelTransit {
    pub room: RoomChannel,
    pub tx: broadcast::Sender<Bytes>,
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
