use bytes::Bytes;
use std::collections::HashMap;
use std::str::FromStr;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task;
use uuid::Uuid;

use crate::shared_lib::config::{PUBLIC_ROOM_ID, PUBLIC_ROOM_NAME};
use crate::shared_lib::types::{RoomChannel, ServerClientMsg};

use super::util::config::ROOM_CAPACITY;
use super::util::types::{
    Client, ClientManagerMsg, DirectChannelTransit, ManagerClientMsg, RoomChannelTransit,
};

pub fn create_manager_task(mut rx_client_manager: mpsc::Receiver<ClientManagerMsg>) {
    task::spawn(async move {
        let mut clients: HashMap<Uuid, Client> = HashMap::new();

        let mut public_room = RoomChannel {
            id: Uuid::from_str(PUBLIC_ROOM_ID).unwrap(),
            name: PUBLIC_ROOM_NAME.into(),
            messages: vec![],
            users: vec![],
        };

        let (tx_public_room, _) = broadcast::channel::<Bytes>(ROOM_CAPACITY);

        loop {
            match rx_client_manager.recv().await.unwrap() {
                ClientManagerMsg::Init(client) => {
                    public_room.users.push(client.user.clone());

                    let room_transit = RoomChannelTransit {
                        room: public_room.clone(),
                        tx: tx_public_room.clone(),
                    };
                    let msg = ManagerClientMsg::JoinRoom(room_transit);
                    client.tx.send(msg).await.unwrap();

                    clients.insert(client.user.id, client);

                    // let msg = ServerClientMsg::RoomUpdate(public_room.clone());
                    // let serialized = bincode::serialize(&msg).unwrap();
                    // tx_public_room.send(serialized.into()).unwrap();
                }

                ClientManagerMsg::ClientDropped(id) => {
                    clients.remove(&id);

                    if let Some(pos) = public_room.users.iter().position(|u| u.id == id) {
                        public_room.users.remove(pos);
                    }

                    let msg = ServerClientMsg::RoomUpdate(public_room.clone());
                    let serialized = bincode::serialize(&msg).unwrap();
                    tx_public_room.send(serialized.into()).unwrap();
                }

                ClientManagerMsg::EstablishDirectComm(c) => {
                    let client = clients.get(&c.payload.to).unwrap();
                    let (tx_ack, rx_ack) = oneshot::channel::<mpsc::Sender<Bytes>>();

                    let transit = DirectChannelTransit {
                        ack: tx_ack,
                        payload: c.payload,
                    };

                    client
                        .tx
                        .send(ManagerClientMsg::EstablishDirectComm(transit))
                        .await
                        .unwrap();
                    let tx_cleint_client = rx_ack.await.unwrap();
                    c.ack.send(tx_cleint_client).unwrap();
                }
            };
        }
    });
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
