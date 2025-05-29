use super::util::types::server_data_types::{
    AuthTransit, ClientPersistenceMsg, CreateRoomResponse, CreateRoomServerTransit, DbRoom, DbUser,
    JoinRoomServerTransit, RegisterDataTransit, UserDataTransit, UserRoomData,
};
use crate::{
    server_lib::util::types::{
        server_data_types::JoinRoommPersistenceResponse, server_error_types::Bt,
    },
    shared_lib::{
        config::{
            PASSWORD_ERROR_MSG, PASSWORD_RE_PATTERN, PUBLIC_ROOM_ID, PUBLIC_ROOM_NAME,
            USERNAME_ERROR_MSG, USERNAME_RE_PATTERN,
        },
        types::{AuthResponse, RegisterResponse, RoomData, User, UserInitData},
    },
};
use log::{debug, warn};
use regex::Regex;
use std::{
    collections::{HashMap, VecDeque},
    str::FromStr,
};
use tokio::{sync::mpsc, task};
use uuid::Uuid;

struct PersistenceTask {
    username_re: Regex,
    password_re: Regex,
    rooms: HashMap<Uuid, DbRoom>,
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
        let mut rooms: HashMap<Uuid, DbRoom> = HashMap::new();
        let users: HashMap<String, DbUser> = HashMap::new();

        let public_room = DbRoom {
            id: Uuid::from_str(PUBLIC_ROOM_ID).unwrap(),
            name: PUBLIC_ROOM_NAME.into(),
            messages: VecDeque::new(),
            users: vec![],
            password: None,
        };

        rooms.insert(public_room.id, public_room.clone());

        Self {
            password_re: Regex::new(PASSWORD_RE_PATTERN).unwrap(),
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
                    ClientPersistenceMsg::CreateRoom(t) => self.handle_create_room(t),
                    ClientPersistenceMsg::JoinRoom(t) => self.handle_join_room(t),
                }
            }
        }
    }

    fn handle_create_room(&mut self, t: CreateRoomServerTransit) {
        if self.rooms.iter().any(|(_, r)| r.name == t.room_name) {
            let res = CreateRoomResponse::Failure(String::from(format!(
                "Room name {} is already taken",
                t.room_name
            )));
            if let Err(err) = t.tx.send(res) {
                debug!(
                    "oneshot receiver dropped before auth finished {err:?} {}",
                    Bt::new()
                )
            }
            return;
        };

        let db_user = match self.users.get_mut(&t.username) {
            None => {
                warn!("Not registered user attepmted to create room");
                let res = CreateRoomResponse::Failure(String::from(format!(
                    "Provided username is not registered"
                )));
                if let Err(err) = t.tx.send(res) {
                    debug!(
                        "oneshot receiver dropped before auth finished {err:?} {}",
                        Bt::new()
                    )
                }
                return;
            }
            Some(u) => u,
        };

        let user = User {
            id: db_user.id,
            username: t.username,
        };

        let new_db_room = DbRoom {
            id: Uuid::new_v4(),
            messages: VecDeque::new(),
            name: t.room_name,
            users: vec![user.clone()],
            password: t.room_password,
        };

        let room_data = RoomData {
            id: new_db_room.id,
            name: new_db_room.name.clone(),
            users: vec![user.clone()],
            users_online: vec![user.clone()],
        };

        debug!("{:?}", new_db_room.password);
        db_user.rooms.push(new_db_room.id);
        self.rooms.insert(new_db_room.id, new_db_room);

        let res = CreateRoomResponse::Success(room_data);
        if let Err(err) = t.tx.send(res) {
            debug!(
                "oneshot auth receiver dropped before auth finished {err:?} {}",
                Bt::new()
            )
        }
    }

    fn handle_join_room(&mut self, t: JoinRoomServerTransit) {
        let room = match self.rooms.iter_mut().find(|(_, r)| r.name == t.room_name) {
            None => {
                let msg = format!(
                    "No room named {} is registered, but you can create one!",
                    t.room_name
                );
                let res = JoinRoommPersistenceResponse::Failure(msg);
                t.tx.send(res).ok();
                return;
            }
            Some((_, r)) => r,
        };

        if room.users.iter().any(|u| u.id == t.user.id) {
            let msg = format!("User {} already is in the room.", t.user.username);
            let res = JoinRoommPersistenceResponse::Failure(msg);
            t.tx.send(res).ok();
            return;
        }
        debug!("privided: {:?}", &t.room_password);
        debug!("required: {:?}", &room.password);

        match (&room.password, &t.room_password) {
            (Some(correct_password), Some(provided_password)) => {
                debug!("1");
                if correct_password != provided_password {
                    debug!("2");
                    let msg = format!("Incorrect room password.");
                    let res = JoinRoommPersistenceResponse::Failure(msg);
                    t.tx.send(res).ok();
                    return;
                }
            }
            (Some(_), None) => {
                debug!("3");
                let msg = format!("Room password required.");
                let res = JoinRoommPersistenceResponse::Failure(msg);
                t.tx.send(res).ok();
                return;
            }
            _ => {
                debug!("4");
            }
        }

        let user = match self.users.get_mut(&t.user.username) {
            None => {
                let msg = format!("No user named {} is registered!", t.user.username);
                let res = JoinRoommPersistenceResponse::Failure(msg);
                t.tx.send(res).ok();
                return;
            }
            Some(u) => u,
        };

        user.rooms.push(room.id);

        let data = RoomData {
            id: room.id,
            name: t.room_name,
            users: room.users.clone(),
            users_online: vec![],
        };

        let res = JoinRoommPersistenceResponse::Success(data);
        t.tx.send(res).ok();
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

        for room_id in &user.rooms {
            match self.rooms.get(room_id) {
                Some(r) => {
                    let room = RoomData {
                        id: r.id,
                        name: r.name.clone(),
                        users: r.users.clone(),
                        users_online: vec![],
                    };
                    user_rooms.push(room);
                }
                None => debug!("Room saved in DbUser does is not persisted!!{}", Bt::new()),
            };
        }

        let data = UserInitData { rooms: user_rooms };

        if let Err(err) = t.tx.send(data) {
            debug!("oneshot register res receiver dropped{err:?} {}", Bt::new());
        };
    }

    fn handle_register(&mut self, t: RegisterDataTransit) {
        if !self.username_re.is_match(&t.data.username) {
            let err_msg = String::from(USERNAME_ERROR_MSG);
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

        if !self.password_re.is_match(&t.data.password)
            || !&t.data.password.chars().any(|c| c.is_lowercase())
            || !&t.data.password.chars().any(|c| c.is_uppercase())
            || !&t.data.password.chars().any(|c| c.is_ascii_digit())
        {
            let res = RegisterResponse::Failure(String::from(PASSWORD_ERROR_MSG));
            if let Err(err) = t.tx.send(res) {
                debug!("oneshot register res receiver dropped{err:?} {}", Bt::new());
            };
            return;
        }

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

    fn handle_user_joined_room(&mut self, t: UserRoomData) {
        if let Some(room) = self.rooms.get_mut(&t.room_id) {
            room.users.push(t.user);
        }
    }
    fn handle_user_left_room(&mut self, t: UserRoomData) {
        if let Some(room) = self.rooms.get_mut(&t.room_id) {
            room.users.retain(|u| u.id != t.user.id);
        }
    }
}
