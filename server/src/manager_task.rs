use crate::util::types::server_error_types::Bt;

use super::util::config::ROOM_CAPACITY;
use super::util::types::server_data_types::{
    Client, ClientManagerMsg, DirectChannelTxTransit, GetRoomTxTransit, IsOnlineTransit,
    ManagerClientMsg, MultipleRoomsUpdateTransit, RoomChannelTxTransit, RoomUpdateTransit,
};
use bytes::Bytes;
use log::{debug, error, info, warn};
use shared::config::PUBLIC_ROOM_ID;
use shared::types::{JoinRoomNotification, RoomData, ServerClientMsg, User};
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
        info!("Manager task running");
        loop {
            let msg = self.rx_client_manager.recv().await.expect(
                "all tx_client_manager transmitters got dropped, one needs to live in server.rs to clone for new connections!!",
            );

            // debug!("msg");

            match msg {
                ClientManagerMsg::ClientConnected(client) => self.handle_client_connected(client),
                ClientManagerMsg::ClientDropped(id) => self.handle_client_dropped(id),
                ClientManagerMsg::GetDirectChannelTx(t) => {
                    self.handle_establish_direct_comm(t).await
                }
                ClientManagerMsg::GetRoomChannelTx(t) => self.handle_get_room_channel_tx(t).await,
                ClientManagerMsg::UserRegistered(user) => self.handle_user_registered(user).await,
                ClientManagerMsg::IsOnline(t) => self.handle_is_online(t),
                ClientManagerMsg::UpdateRoom(t) => self.handle_update_room(t),
                ClientManagerMsg::UpdateMultipleRooms(t) => self.handle_update_multiple_rooms(t),
            }
        }
    }

    fn handle_update_room(&self, mut t: RoomUpdateTransit) {
        self.update_room_online_users(&mut t.room);
        t.tx_ack.send(t.room).ok();
    }

    fn handle_update_multiple_rooms(&self, mut t: MultipleRoomsUpdateTransit) {
        for room in &mut t.rooms {
            self.update_room_online_users(room);
        }
        t.tx_ack.send(t.rooms).ok();
    }

    async fn handle_get_room_channel_tx(&mut self, t: RoomChannelTxTransit) {
        for user in &t.room_users {
            if let Some(client) = self.connected_users.get(&user.id) {
                let transit = GetRoomTxTransit {
                    room_id: t.room_id,
                    tx_ack: t.ack,
                };
                let msg = ManagerClientMsg::GetRoomTransmitter(transit);
                if let Err(err) = client.tx.send(msg).await {
                    warn!(
                        "connected clients hasmap is not synhronized with running client_tasts!!, {} {}",
                        err,
                        Bt::new()
                    )
                };
                return;
            };
        }

        let (room_tx, _) = broadcast::channel(ROOM_CAPACITY);

        t.ack.send(room_tx).ok();
    }

    fn handle_is_online(&self, t: IsOnlineTransit) {
        let is_online = self
            .connected_users
            .iter()
            .any(|(_, c)| c.user.username == t.username);

        if t.ack.send(is_online).is_err() {
            debug!("oneshot acknowledge receiver dropped {}", Bt::new());
        };
    }

    fn handle_client_connected(&mut self, client: Client) {
        self.connected_users.insert(client.user.id, client);
    }

    fn handle_client_dropped(&mut self, id: Uuid) {
        self.connected_users.remove(&id);
    }

    async fn handle_establish_direct_comm(&mut self, t: DirectChannelTxTransit) {
        let client = match self.connected_users.get(&t.payload.to) {
            Some(c) => c,
            None => {
                warn!("Client not found among online clients, {}", Bt::new());
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
                Bt::new()
            );
        };
    }

    async fn handle_user_registered(&mut self, user: User) {
        for (_, client) in &self.connected_users {
            let (tx_ack, rx_ack) = oneshot::channel::<broadcast::Sender<Bytes>>();

            let transit = GetRoomTxTransit {
                room_id: Uuid::from_str(PUBLIC_ROOM_ID).unwrap(),
                tx_ack,
            };
            let msg = ManagerClientMsg::GetRoomTransmitter(transit);
            client.tx.send(msg).await.unwrap();

            let tx = rx_ack.await.unwrap();

            let transit = JoinRoomNotification {
                room_id: Uuid::from_str(PUBLIC_ROOM_ID).unwrap(),
                user,
            };
            let msg = ServerClientMsg::UserJoinedRoom(transit);
            let serialized = bincode::serialize(&msg).unwrap();

            tx.send(serialized.into()).unwrap();
            break;
        }
    }

    fn update_room_online_users(&self, room: &mut RoomData) {
        room.users_online = room
            .users
            .iter()
            .cloned()
            .filter(|u| self.connected_users.get(&u.id).is_some())
            .collect();
    }
}
