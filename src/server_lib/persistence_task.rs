use super::util::types::server_data_types::{ClientPersistenceMsg, DbUser, UserServerData};
use crate::{
    server_lib::util::types::server_error_types::Bt,
    shared_lib::{
        config::{PUBLIC_ROOM_ID, PUBLIC_ROOM_NAME, USERNAME_RE_PATTERN},
        types::{AuthResponse, RegisterResponse, RoomChannel, User},
    },
};
use log::debug;
use regex::Regex;
use std::{
    collections::{HashMap, VecDeque},
    str::FromStr,
    vec,
};
use tokio::{sync::mpsc, task};
use uuid::Uuid;

pub fn spawn_persistence_task(mut rx_client_persistence: mpsc::Receiver<ClientPersistenceMsg>) {
    let username_pattern = USERNAME_RE_PATTERN;
    let username_re = Regex::new(username_pattern).unwrap();

    task::spawn(async move {
        let mut rooms = HashMap::new();
        let mut users: HashMap<String, DbUser> = HashMap::new();

        let public_room = RoomChannel {
            id: Uuid::from_str(PUBLIC_ROOM_ID).unwrap(),
            name: PUBLIC_ROOM_NAME.into(),
            messages: VecDeque::new(),
            users: vec![],
        };

        rooms.insert(public_room.id, public_room.clone());

        loop {
            if let Some(msg) = rx_client_persistence.recv().await {
                match msg {
                    ClientPersistenceMsg::GetUserData(t) => {
                        let user = match users.get(&t.user.username) {
                            Some(user) => user,
                            None => {
                                debug!("no user with \"{}\"", &t.user.username);
                                continue;
                            }
                        };

                        let mut user_rooms = vec![];

                        for room_id in user.rooms.iter() {
                            match rooms.get(room_id) {
                                Some(r) => {
                                    user_rooms.push(r.clone());
                                }
                                None => debug!(
                                    "Room saved in DbUser does is not persisted!!{}",
                                    Bt::new()
                                ),
                            };
                        }

                        let data = UserServerData { rooms: user_rooms };

                        if let Err(err) = t.tx.send(data) {
                            debug!("oneshot register res receiver dropped{err:?} {}", Bt::new());
                        };
                    }
                    ClientPersistenceMsg::UserJoinedRoom(t) => {
                        let room = match rooms.get_mut(&t.room_id) {
                            None => continue,
                            Some(r) => r,
                        };

                        room.users.push(t.user);
                    }
                    ClientPersistenceMsg::UserLeftRoom(t) => {
                        let room = match rooms.get_mut(&t.room_id) {
                            None => continue,
                            Some(r) => r,
                        };

                        room.users.retain(|u| u.id != t.user.id);
                    }
                    ClientPersistenceMsg::Register(t) => {
                        if !username_re.is_match(&t.data.username) {
                            let err_msg =  String::from("Username must start with a letter, not contain special characters ouside of \"_\" and have length between 7 to 29");
                            let res = RegisterResponse::Failure(err_msg);
                            if let Err(err) = t.tx.send(res) {
                                debug!(
                                    "oneshot register res receiver dropped{err:?} {}",
                                    Bt::new()
                                );
                            };
                            continue;
                        };

                        if users.contains_key(t.data.username.as_str()) {
                            let res =
                                RegisterResponse::Failure(String::from("Username already taken"));
                            if let Err(err) = t.tx.send(res) {
                                debug!(
                                    "oneshot register res receiver dropped{err:?} {}",
                                    Bt::new()
                                );
                            };
                            continue;
                        };

                        let new_db_user = DbUser {
                            id: Uuid::new_v4(),
                            password: t.data.password,
                            username: t.data.username,
                            rooms: vec![public_room.id],
                        };

                        let new_user = User {
                            id: new_db_user.id,
                            username: new_db_user.username.clone(),
                        };

                        let public_room = rooms
                            .get_mut(&Uuid::from_str(PUBLIC_ROOM_ID).unwrap())
                            .unwrap();

                        public_room.users.push(new_user.clone());

                        let res = RegisterResponse::Success(new_user);
                        users.insert(new_db_user.username.clone(), new_db_user);

                        if let Err(err) = t.tx.send(res) {
                            debug!("oneshot register res receiver dropped{err:?} {}", Bt::new());
                        };
                    }
                    ClientPersistenceMsg::Authenticate(t) => {
                        let user = match users.get(&t.data.username) {
                            Some(c) => c,
                            None => {
                                let res = AuthResponse::Failure(format!(
                                    "No account with username {} found, register first",
                                    t.data.username
                                ));
                                if let Err(err) = t.tx.send(res) {
                                    debug!(
                                        "oneshot auth receiver dropped before auth finished {err:?} {}",
                                        Bt::new()
                                    )
                                }
                                continue;
                            }
                        };
                        let res = if t.data.password == user.password {
                            AuthResponse::Success(User {
                                username: t.data.username,
                                id: user.id,
                            })
                        } else {
                            AuthResponse::Failure(format!("Incorrect password"))
                        };

                        if let Err(err) = t.tx.send(res) {
                            debug!(
                                "oneshot auth receiver dropped before auth finished {err:?} {}",
                                Bt::new()
                            )
                        }
                    }
                }
            }
        }
    });
}
