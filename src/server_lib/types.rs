use bytes::Bytes;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

#[derive(Debug)]
pub struct ChannelTransitPayload {
    pub tx_client_client: mpsc::Sender<Bytes>,
    pub from: Uuid,
    pub to: Uuid,
}

#[derive(Debug)]
pub struct ChannelTransit {
    pub payload: ChannelTransitPayload,
    pub ack: oneshot::Sender<mpsc::Sender<Bytes>>,
}

pub enum ClientToManagerMessage {
    Init(mpsc::Sender<ManagerToClientMessage>, Uuid),
    ClientDropped(Uuid),
    EstablishDirectComm(ChannelTransit),
}

#[derive(Debug)]
pub enum ManagerToClientMessage {
    EstablishDirectComm(ChannelTransit),
}
