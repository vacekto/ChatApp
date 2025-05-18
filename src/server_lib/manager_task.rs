use super::util::config::ROOM_CAPACITY;
use super::util::types::{Client, ClientManagerMsg, GetRoomTransmitterTransit, ManagerClientMsg};
use crate::server_lib::util::server_functions::get_location;
use log::{debug, error, warn};
use std::collections::HashMap;
use tokio::sync::{broadcast, mpsc};
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
                ClientManagerMsg::CheckUsername(data) => {
                    let res = connected_users
                        .iter()
                        .find(|c| c.1.user.username == data.username)
                        .is_some();

                    data.tx.send(res).ok();
                }

                ClientManagerMsg::ClientConnected(client) => {
                    connected_users.insert(client.user.id, client);
                }
                ClientManagerMsg::GetOnlineUsers(t) => {
                    let users = connected_users.values().map(|c| c.user.clone()).collect();
                    if t.ack.send(users).is_err() {
                        error!(
                            "oneshot rx dropped before receiving data: {}",
                            get_location()
                        );
                    };
                }
                ClientManagerMsg::ClientDropped(id) => {
                    connected_users.remove(&id);
                }
                ClientManagerMsg::EstablishRoomComm(t) => {
                    for user in &t.room_users {
                        if let Some(client) = connected_users.get(&user.id) {
                            let transit = GetRoomTransmitterTransit {
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
            };
        }
    });
}
