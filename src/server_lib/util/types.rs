use bytes::Bytes;
use tokio::sync::{broadcast, mpsc, oneshot};
use uuid::Uuid;

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

pub enum ClientToManagerMessage {
    Init(mpsc::Sender<ManagerToClientMsg>, Uuid),
    ClientDropped(Uuid),
    EstablishDirectComm(DirectChannelTransit),
}

pub enum ManagerToClientMsg {
    EstablishDirectComm(DirectChannelTransit),
    JoinRoom(RoomChannelTransit),
}

pub struct RoomChannelTransit {
    pub room_id: Uuid,
    pub tx: broadcast::Sender<Bytes>,
}

pub struct MpscChannel<T = Bytes, K = Bytes> {
    pub tx: mpsc::Sender<T>,
    pub rx: mpsc::Receiver<K>,
}
pub struct BroadcastChannel<T = Bytes, K = Bytes> {
    pub tx: broadcast::Sender<T>,
    pub rx: broadcast::Receiver<K>,
}
