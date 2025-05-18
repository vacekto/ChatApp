use super::util::types::{ClientPersistenceMsg, PersistedUserData};
use crate::shared_lib::{
    config::{PUBLIC_ROOM_ID, PUBLIC_ROOM_NAME},
    types::RoomChannel,
};
use log::debug;
use std::{
    collections::{HashMap, VecDeque},
    str::FromStr,
    vec,
};
use tokio::{sync::mpsc, task};
use uuid::Uuid;

pub fn spawn_persistence_task(
    mut rx_client_persistence: mpsc::Receiver<ClientPersistenceMsg>,
    // tx_persistance_manager: mpsc::Sender<PersistenceManagerMsg>,
) {
    task::spawn(async move {
        let public_room = RoomChannel {
            id: Uuid::from_str(PUBLIC_ROOM_ID).unwrap(),
            name: PUBLIC_ROOM_NAME.into(),
            messages: VecDeque::new(),
            users: vec![],
        };

        let mut rooms = HashMap::new();
        rooms.insert(public_room.id, public_room.clone());

        loop {
            if let Some(msg) = rx_client_persistence.recv().await {
                match msg {
                    ClientPersistenceMsg::GetUserData(t) => {
                        let (_, public_room) = rooms
                            .iter()
                            .find(|(_, r)| r.id == Uuid::from_str(PUBLIC_ROOM_ID).unwrap())
                            .unwrap();

                        let data = PersistedUserData {
                            rooms: vec![public_room.clone()],
                        };
                        t.tx.send(data).unwrap();
                    }
                    ClientPersistenceMsg::UserJoinedRoom(room_data) => {
                        let room = match rooms.get_mut(&room_data.room_id) {
                            None => continue,
                            Some(r) => r,
                        };

                        room.users.push(room_data.user);
                    }
                    ClientPersistenceMsg::UserLeftRoom(t) => {
                        let room = match rooms.get_mut(&t.room_id) {
                            None => continue,
                            Some(r) => r,
                        };

                        room.users.retain(|u| u.id != t.user.id);
                    }
                }
            }
        }
    });
}
