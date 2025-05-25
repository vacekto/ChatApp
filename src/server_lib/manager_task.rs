use super::util::config::ROOM_CAPACITY;
use super::util::types::server_data_types::{
    Client, ClientManagerMsg, GetRoomTxTransit, ManagerClientMsg,
};
use crate::server_lib::util::server_functions::get_location;
use crate::server_lib::util::types::server_error_types::Bt;
use crate::shared_lib::config::PUBLIC_ROOM_ID;
use crate::shared_lib::types::{RoomUpdateTransit, ServerClientMsg, TuiRoom};
use bytes::Bytes;
use log::{debug, error, warn};
use std::collections::HashMap;
use std::str::FromStr;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task;
use uuid::Uuid;

pub fn spawn_manager_task(mut rx_client_manager: mpsc::Receiver<ClientManagerMsg>) {
    task::spawn(async move {
        let mut connected_users: HashMap<Uuid, Client> = HashMap::new();

        'manager_loop: loop {
            let msg = rx_client_manager.recv().await.expect(
                "all tx_client_manager transmitters got dropped, one needs to live in server.rs to clone for new connections!!",
            );

            match msg {
                ClientManagerMsg::ClientConnected(client) => {
                    connected_users.insert(client.user.id, client);
                }
                ClientManagerMsg::ClientDropped(id) => {
                    connected_users.remove(&id);
                }

                ClientManagerMsg::EstablishRoomComm(t) => {
                    for user in &t.room_users {
                        if let Some(client) = connected_users.get(&user.id) {
                            let transit = GetRoomTxTransit {
                                room_id: t.room_id,
                                tx_ack: t.ack,
                            };
                            let msg = ManagerClientMsg::GetRoomTransmitter(transit);
                            if let Err(err) = client.tx.send(msg).await {
                                warn!("connected clients hasmap is not synhronized with running client_tasts!!, {} {}", err, get_location())
                            };
                            continue 'manager_loop;
                        };
                    }

                    let (room_tx, _) = broadcast::channel(ROOM_CAPACITY);
                    t.ack.send(room_tx).ok();
                }
                ClientManagerMsg::EstablishDirectComm(c) => {
                    let client = match connected_users.get(&c.payload.to) {
                        Some(c) => c,
                        None => {
                            warn!("Client not found among online clients, {}", get_location());
                            continue;
                        }
                    };

                    if client
                        .tx
                        .send(ManagerClientMsg::EstablishDirectComm(c))
                        .await
                        .is_err()
                    {
                        error!(
                            "error during establishing direct communication, {}",
                            get_location()
                        );
                    };
                }
                ClientManagerMsg::GetConnectedUsers(t) => {
                    let mut data = vec![];
                    for room in t.rooms {
                        let mut users_online = vec![];
                        for user in room
                            .users
                            .iter()
                            .filter(|u| connected_users.get(&u.id).is_some())
                        {
                            users_online.push(user.clone());
                        }

                        let tui_room = TuiRoom {
                            id: room.id,
                            messages: room.messages.clone(),
                            name: room.name.clone(),
                            users: room.users.clone(),
                            users_online,
                        };
                        data.push(tui_room);
                    }

                    if t.tx_ack.send(data).is_err() {
                        debug!("oneshot acknowledge receiver dropped {}", Bt::new());
                    };
                }
                ClientManagerMsg::UserRegistered(user) => {
                    for (_, client) in &connected_users {
                        let (tx_ack, rx_ack) = oneshot::channel::<broadcast::Sender<Bytes>>();

                        let transit = GetRoomTxTransit {
                            room_id: Uuid::from_str(PUBLIC_ROOM_ID).unwrap(),
                            tx_ack,
                        };
                        let msg = ManagerClientMsg::GetRoomTransmitter(transit);
                        client.tx.send(msg).await.unwrap();

                        let tx = rx_ack.await.unwrap();

                        let transit = RoomUpdateTransit {
                            room_id: Uuid::from_str(PUBLIC_ROOM_ID).unwrap(),
                            user,
                        };
                        let msg = ServerClientMsg::UserJoinedRoom(transit);
                        let serialized = bincode::serialize(&msg).unwrap();

                        tx.send(serialized.into()).unwrap();
                        break;
                    }
                }
            };
        }
    });
}
