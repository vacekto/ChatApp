use std::collections::HashMap;

use bytes::Bytes;
use futures::SinkExt;
use tokio::{
    net::TcpStream,
    sync::{mpsc, oneshot},
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use uuid::Uuid;

use crate::shared_lib::types::InitClientData;

use super::util::{
    errors::DataParsingError,
    types::{ClientToManagerMessage, DirectChannelTransit, ManagerToClientMessage},
};

pub async fn create_manager_task(mut rx_client_manager: mpsc::Receiver<ClientToManagerMessage>) {
    let mut clients: HashMap<Uuid, mpsc::Sender<ManagerToClientMessage>> = HashMap::new();

    loop {
        match rx_client_manager.recv().await.unwrap() {
            ClientToManagerMessage::Init(tx_manager_client, id) => {
                clients.insert(id, tx_manager_client);
            }
            ClientToManagerMessage::ClientDropped(id) => {
                clients.remove(&id);
            }
            ClientToManagerMessage::EstablishDirectComm(c) => {
                let tx_target = clients.get(&c.payload.to).unwrap();
                let (tx_ack, rx_ack) = oneshot::channel::<mpsc::Sender<Bytes>>();

                let transit = DirectChannelTransit {
                    ack: tx_ack,
                    payload: c.payload,
                };

                tx_target
                    .send(ManagerToClientMessage::EstablishDirectComm(transit))
                    .await
                    .unwrap();
                let tx_cleint_client = rx_ack.await.unwrap();
                c.ack.send(tx_cleint_client).unwrap();
            }
        };
    }
}

// fn create_room_comm_task(
//     mut rx_client_room: broadcast::Receiver<Bytes>,
//     tx_comm_client: mpsc::Sender<Bytes>,
// ) {
//     task::spawn(async move {
//         while let Ok(data) = rx_client_room.recv().await {
//             tx_comm_client.send(data).await.unwrap();
//         }
//     });
// }

pub async fn init_client(tcp: &mut TcpStream) -> Result<InitClientData, DataParsingError> {
    let itid_data = InitClientData { id: Uuid::new_v4() };

    let mut framed_tcp = Framed::new(tcp, LengthDelimitedCodec::new());
    let encoded = bincode::serialize(&itid_data)?;
    framed_tcp.send(encoded.into()).await?;
    Ok(itid_data)
}
