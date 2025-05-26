use super::util::config::ROOM_CAPACITY;
use super::util::types::server_data_types::{
    Client, ClientManagerMsg, EstablishDirectCommTransit, EstablishRoomCommTransit,
    GetConnectedUsersTransit, GetRoomTxTransit, ManagerClientMsg,
};
use crate::server_lib::util::server_functions::get_location;
use crate::server_lib::util::types::server_error_types::Bt;
use crate::shared_lib::config::PUBLIC_ROOM_ID;
use crate::shared_lib::types::{RoomUpdateTransit, ServerClientMsg, TuiRoom, User};
use bytes::Bytes;
use log::{debug, error, warn};
use std::collections::HashMap;
use std::str::FromStr;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task;
use uuid::Uuid;

struct ManagerTask {
    rx_client_manager: mpsc::Receiver<ClientManagerMsg>,
    connected_users: HashMap<Uuid, Client>,
}

pub fn spawn_manager_task(rx_client_persistence: mpsc::Receiver<ClientManagerMsg>) {
    task::spawn(async move {
        let mut handler = ManagerTask::new(rx_client_persistence);
        handler.run().await;
    });
}

impl ManagerTask {
    fn new(rx_client_manager: mpsc::Receiver<ClientManagerMsg>) -> Self {
        Self {
            rx_client_manager,
            connected_users: HashMap::new(),
        }
    }

    async fn run(&mut self) {
        loop {
            let msg = self.rx_client_manager.recv().await.expect(
                "all tx_client_manager transmitters got dropped, one needs to live in server.rs to clone for new connections!!",
            );

            match msg {
                ClientManagerMsg::ClientConnected(client) => self.handle_client_connected(client),
                ClientManagerMsg::ClientDropped(id) => self.handle_client_dropped(id),
                ClientManagerMsg::EstablishDirectComm(t) => {
                    self.handle_establish_direct_comm(t).await
                }
                ClientManagerMsg::EstablishRoomComm(t) => self.handle_establish_room_comm(t).await,
                ClientManagerMsg::GetConnectedUsers(t) => self.handle_get_connected_users(t),
                ClientManagerMsg::UserRegistered(user) => self.handl_user_registered(user).await,
            }
        }
    }

    fn handle_client_connected(&mut self, client: Client) {
        self.connected_users.insert(client.user.id, client);
    }

    fn handle_client_dropped(&mut self, id: Uuid) {
        self.connected_users.remove(&id);
    }

    async fn handle_establish_direct_comm(&mut self, t: EstablishDirectCommTransit) {
        let client = match self.connected_users.get(&t.payload.to) {
            Some(c) => c,
            None => {
                warn!("Client not found among online clients, {}", get_location());
                return;
            }
        };

        if client
            .tx
            .send(ManagerClientMsg::EstablishDirectComm(t))
            .await
            .is_err()
        {
            error!(
                "error during establishing direct communication, {}",
                get_location()
            );
        };
    }

    async fn handle_establish_room_comm(&mut self, t: EstablishRoomCommTransit) {
        for user in &t.room_users {
            if let Some(client) = self.connected_users.get(&user.id) {
                let transit = GetRoomTxTransit {
                    room_id: t.room_id,
                    tx_ack: t.ack,
                };
                let msg = ManagerClientMsg::GetRoomTransmitter(transit);
                if let Err(err) = client.tx.send(msg).await {
                    warn!("connected clients hasmap is not synhronized with running client_tasts!!, {} {}", err, get_location())
                };
                return;
            };
        }

        let (room_tx, _) = broadcast::channel(ROOM_CAPACITY);
        t.ack.send(room_tx).ok();
    }

    fn handle_get_connected_users(&mut self, t: GetConnectedUsersTransit) {
        let mut data = vec![];
        for room in t.rooms {
            let mut users_online = vec![];
            for user in room
                .users
                .iter()
                .filter(|u| self.connected_users.get(&u.id).is_some())
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

    async fn handl_user_registered(&mut self, user: User) {
        for (_, client) in &self.connected_users {
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
}
