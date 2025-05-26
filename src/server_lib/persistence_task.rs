use super::util::types::server_data_types::{
    AuthTransit, ClientPersistenceMsg, DbUser, RegisterDataTransit, UserDataTransit,
    UserRoomTransit, UserServerData,
};
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
};
use tokio::{sync::mpsc, task};
use uuid::Uuid;
struct PersistenceTask {
    username_re: Regex,
    rooms: HashMap<Uuid, RoomChannel>,
    users: HashMap<String, DbUser>,
    rx_client_persistence: mpsc::Receiver<ClientPersistenceMsg>,
}

pub fn spawn_persistence_task(rx_client_persistence: mpsc::Receiver<ClientPersistenceMsg>) {
    task::spawn(async move {
        let mut handler = PersistenceTask::new(rx_client_persistence);
        handler.run().await;
    });
}

impl PersistenceTask {
    fn new(rx_client_persistence: mpsc::Receiver<ClientPersistenceMsg>) -> Self {
        let mut rooms = HashMap::new();
        let mut users: HashMap<String, DbUser> = HashMap::new();

        let public_room = RoomChannel {
            id: Uuid::from_str(PUBLIC_ROOM_ID).unwrap(),
            name: PUBLIC_ROOM_NAME.into(),
            messages: VecDeque::new(),
            users: vec![],
        };

        rooms.insert(public_room.id, public_room.clone());

        let default_user = DbUser {
            id: Uuid::new_v4(),
            password: "Cosikdosi1".into(),
            username: "Cosikdosi1".into(),
            rooms: vec![public_room.id],
        };

        users.insert("Cosikdosi1".into(), default_user);

        Self {
            username_re: Regex::new(USERNAME_RE_PATTERN).unwrap(),
            rooms,
            users,
            rx_client_persistence,
        }
    }

    async fn run(&mut self) {
        loop {
            if let Some(msg) = self.rx_client_persistence.recv().await {
                match msg {
                    ClientPersistenceMsg::Authenticate(t) => self.handle_auth(t),
                    ClientPersistenceMsg::Register(t) => self.handle_register(t),
                    ClientPersistenceMsg::GetUserData(t) => self.get_user_data(t),
                    ClientPersistenceMsg::UserJoinedRoom(t) => self.handle_user_joined_room(t),
                    ClientPersistenceMsg::UserLeftRoom(t) => self.handle_user_left_room(t),
                }
            }
        }
    }

    fn handle_auth(&mut self, t: AuthTransit) {
        let user = match self.users.get(&t.data.username) {
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
                return;
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

    fn get_user_data(&mut self, t: UserDataTransit) {
        let user = match self.users.get(&t.user.username) {
            Some(user) => user,
            None => {
                debug!("no user with \"{}\"", &t.user.username);
                return;
            }
        };

        let mut user_rooms = vec![];

        for room_id in user.rooms.iter() {
            match self.rooms.get(room_id) {
                Some(r) => {
                    user_rooms.push(r.clone());
                }
                None => debug!("Room saved in DbUser does is not persisted!!{}", Bt::new()),
            };
        }

        let data = UserServerData { rooms: user_rooms };

        if let Err(err) = t.tx.send(data) {
            debug!("oneshot register res receiver dropped{err:?} {}", Bt::new());
        };
    }

    fn handle_register(&mut self, t: RegisterDataTransit) {
        if !self.username_re.is_match(&t.data.username) {
            let err_msg =  String::from("Username must start with a letter, not contain special characters ouside of \"_\" and have length between 7 to 29");
            let res = RegisterResponse::Failure(err_msg);
            if let Err(err) = t.tx.send(res) {
                debug!("oneshot register res receiver dropped{err:?} {}", Bt::new());
            };
            return;
        };

        if self.users.contains_key(t.data.username.as_str()) {
            let res = RegisterResponse::Failure(String::from("Username already taken"));
            if let Err(err) = t.tx.send(res) {
                debug!("oneshot register res receiver dropped{err:?} {}", Bt::new());
            };
            return;
        };

        let public_room_id = Uuid::from_str(PUBLIC_ROOM_ID).unwrap();

        let new_db_user = DbUser {
            id: Uuid::new_v4(),
            password: t.data.password,
            username: t.data.username,
            rooms: vec![public_room_id],
        };

        let new_user = User {
            id: new_db_user.id,
            username: new_db_user.username.clone(),
        };

        let public_room = self.rooms.get_mut(&public_room_id).unwrap();

        public_room.users.push(new_user.clone());

        let res = RegisterResponse::Success(new_user);
        self.users.insert(new_db_user.username.clone(), new_db_user);

        if let Err(err) = t.tx.send(res) {
            debug!("oneshot register res receiver dropped{err:?} {}", Bt::new());
        };
    }

    fn handle_user_joined_room(&mut self, t: UserRoomTransit) {
        if let Some(room) = self.rooms.get_mut(&t.room_id) {
            room.users.push(t.user);
        }
    }
    fn handle_user_left_room(&mut self, t: UserRoomTransit) {
        if let Some(room) = self.rooms.get_mut(&t.room_id) {
            room.users.retain(|u| u.id != t.user.id);
        }
    }
}
