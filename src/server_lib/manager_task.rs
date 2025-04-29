use anyhow::anyhow;
use bytes::Bytes;
use std::collections::HashMap;
use std::str::FromStr;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task;
use uuid::Uuid;

use crate::shared_lib::config::{PUBLIC_ROOM_ID, PUBLIC_ROOM_NAME};
use crate::shared_lib::types::{RoomChannel, ServerTuiMsg};

use super::util::config::{log, ROOM_CAPACITY};
use super::util::errors::DataParsingError;
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
            let msg = rx_client_manager.recv().await.expect(
                "all tx_client_manager got dropped, should be at saved and cloned from server.rs",
            );

            match msg {
                ClientManagerMsg::Init(client) => {
                    public_room.users.push(client.user.clone());

                    let room_transit = RoomChannelTransit {
                        room: public_room.clone(),
                        tx: tx_public_room.clone(),
                    };

                    let msg = ManagerClientMsg::JoinRoom(room_transit);

                    if let Err(err) = client.tx.send(msg).await {
                        log(err.into(), Some("initiating client"));
                    };

                    clients.insert(client.user.id, client);
                }

                ClientManagerMsg::ClientDropped(id) => {
                    clients.remove(&id);

                    if let Some(pos) = public_room.users.iter().position(|u| u.id == id) {
                        public_room.users.remove(pos);
                    };

                    let msg = ServerTuiMsg::RoomUpdate(public_room.clone());
                    let serialized = match bincode::serialize(&msg) {
                        Ok(v) => v,
                        Err(err) => {
                            log(
                                DataParsingError::from(err).into(),
                                Some("ServerTuiMsg bincode parsing"),
                            );
                            continue;
                        }
                    };

                    tx_public_room.send(serialized.into()).unwrap();
                }

                ClientManagerMsg::EstablishDirectComm(c) => {
                    let client = match clients.get(&c.payload.to) {
                        Some(c) => c,
                        None => {
                            log(anyhow!("Client not found in clients list"), None);
                            continue;
                        }
                    };
                    let (tx_ack, rx_ack) = oneshot::channel::<mpsc::Sender<Bytes>>();

                    let transit = DirectChannelTransit {
                        ack: tx_ack,
                        payload: c.payload,
                    };

                    if let Err(err) = client
                        .tx
                        .send(ManagerClientMsg::EstablishDirectComm(transit))
                        .await
                    {
                        log(err.into(), Some("establishing direct communication"));
                    };

                    let tx_cleint_client = match rx_ack.await {
                        Ok(v) => v,
                        Err(err) => {
                            log(err.into(), Some("establishing direct communication"));
                            continue;
                        }
                    };

                    if c.ack.send(tx_cleint_client).is_err() {
                        log(anyhow!("establishing direct communication"), None);
                    };
                }
            };
        }
    });
}
