use super::util::types::server_data_types::{
    AuthTransit, ClientPersistenceMsg, CreateRoomResponse, CreateRoomServerTransit, DbRoom, DbUser,
    JoinRoomServerTransit, RegisterDataTransit, UserDataTransit, UserRoomData,
};
use crate::{
    server_lib::util::{
        server_functions::{bson_to_uuid, uuid_to_bson},
        types::{server_data_types::JoinRoommPersistenceResponse, server_error_types::Bt},
    },
    shared_lib::{
        config::{
            PASSWORD_ERROR_MSG, PASSWORD_RE_PATTERN, PUBLIC_ROOM_ID, PUBLIC_ROOM_NAME,
            USERNAME_ERROR_MSG, USERNAME_RE_PATTERN,
        },
        types::{AuthResponse, RegisterResponse, RoomData, User, UserInitData},
    },
};
use anyhow::{anyhow, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2, PasswordHash, PasswordVerifier,
};
use futures::StreamExt;
use log::error;
use log::{debug, info, warn};
use mongodb::{
    bson::{doc, Document},
    options::ClientOptions,
    Client, Collection,
};
use regex::Regex;
use std::str::FromStr;
use tokio::{sync::mpsc, task};
use uuid::Uuid;

struct PersistenceTask {
    username_re: Regex,
    pwd_re: Regex,
    rx_client_persistence: mpsc::Receiver<ClientPersistenceMsg>,
    users_collection: Collection<DbUser>,
    rooms_collection: Collection<DbRoom>,
}

pub fn spawn_persistence_task(rx_client_persistence: mpsc::Receiver<ClientPersistenceMsg>) {
    task::spawn(async move {
        let mut handler = match PersistenceTask::new(rx_client_persistence).await {
            Ok(handler) => handler,
            Err(err) => {
                error!("Error while connecting to MongoDB: {}", err);
                return;
            }
        };
        handler.run().await;
    });
}

impl PersistenceTask {
    async fn new(rx_client_persistence: mpsc::Receiver<ClientPersistenceMsg>) -> Result<Self> {
        let mongo_addr = std::env::var("DB_URL").unwrap();

        let options = ClientOptions::parse(mongo_addr).await?;

        let client = Client::with_options(options)?;
        let db = client.database("chatapp");

        let users_collection = db.collection::<DbUser>("User");
        let rooms_collection = db.collection::<DbRoom>("Room");

        let bson_id = uuid_to_bson(Uuid::from_str(PUBLIC_ROOM_ID)?);

        let filter = doc! { "id": bson_id };
        let room = rooms_collection.find_one(filter).await?;

        let public_room = DbRoom {
            id: uuid_to_bson(Uuid::from_str(PUBLIC_ROOM_ID)?),
            name: PUBLIC_ROOM_NAME.into(),
            user_ids: vec![],
            pwd: None,
        };

        if room.is_none() {
            rooms_collection.insert_one(public_room).await?;
        }

        Ok(Self {
            pwd_re: Regex::new(PASSWORD_RE_PATTERN)?,
            username_re: Regex::new(USERNAME_RE_PATTERN)?,
            rx_client_persistence,
            rooms_collection,
            users_collection,
        })
    }

    async fn run(&mut self) {
        info!("Persistence task running");
        loop {
            if let Some(msg) = self.rx_client_persistence.recv().await {
                let users = self.users_collection.clone();
                let rooms = self.rooms_collection.clone();
                let pwd_re = self.pwd_re.clone();
                let username_re = self.username_re.clone();

                task::spawn(async move {
                    let res = match msg {
                        ClientPersistenceMsg::Authenticate(t) => {
                            PersistenceTask::handle_auth(t, users).await
                        }
                        ClientPersistenceMsg::Register(t) => {
                            PersistenceTask::handle_register(t, users, rooms, pwd_re, username_re)
                                .await
                        }
                        ClientPersistenceMsg::GetUserData(t) => {
                            PersistenceTask::get_user_data(t, users, rooms).await
                        }
                        ClientPersistenceMsg::UserJoinedRoom(t) => {
                            PersistenceTask::handle_user_joined_room(t, users, rooms).await
                        }
                        ClientPersistenceMsg::UserLeftRoom(t) => {
                            PersistenceTask::handle_user_left_room(t, users, rooms).await
                        }
                        ClientPersistenceMsg::CreateRoom(t) => {
                            PersistenceTask::handle_create_room(t, users, rooms).await
                        }
                        ClientPersistenceMsg::JoinRoom(t) => {
                            PersistenceTask::handle_join_room(t, users, rooms).await
                        }
                    };

                    if let Err(err) = res {
                        error!("Persistence task error: {err}");
                    }
                });
            }
        }
    }

    async fn handle_create_room(
        t: CreateRoomServerTransit,
        users_collection: Collection<DbUser>,
        rooms_collection: Collection<DbRoom>,
    ) -> Result<()> {
        let filter = doc! { "name": t.room_name.clone() };
        let room_res = rooms_collection.find_one(filter).await?;

        if room_res.is_some() {
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
            return Ok(());
        };

        let filter = doc! { "username": &t.username.to_string() };
        let db_user = match users_collection.find_one(filter).await? {
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
                return Ok(());
            }
            Some(u) => u,
        };

        let user = User {
            id: bson_to_uuid(&db_user.id).ok_or(anyhow!("expected uuid value"))?,
            username: t.username,
        };

        let new_db_room = DbRoom {
            id: uuid_to_bson(Uuid::new_v4()),
            name: t.room_name,
            user_ids: vec![db_user.id.clone()],
            pwd: t.room_pwd,
        };

        let room_data = RoomData {
            id: bson_to_uuid(&new_db_room.id).ok_or(anyhow!("expected uuid value"))?,
            name: new_db_room.name.clone(),
            users: vec![user.clone()],
            users_online: vec![user.clone()],
        };

        let filter = doc! { "id": db_user.id };
        let update = doc! { "$push": { "room_ids": new_db_room.id.clone() } };

        rooms_collection.insert_one(new_db_room).await?;
        users_collection.find_one_and_update(filter, update).await?;

        let res = CreateRoomResponse::Success(room_data);
        if let Err(err) = t.tx.send(res) {
            debug!(
                "oneshot auth receiver dropped before auth finished {err:?} {}",
                Bt::new()
            )
        }

        Ok(())
    }

    async fn handle_join_room(
        t: JoinRoomServerTransit,
        users_collection: Collection<DbUser>,
        rooms_collection: Collection<DbRoom>,
    ) -> Result<()> {
        let filter = doc! { "name": &t.room_name };
        let res = rooms_collection.find_one(filter).await?;

        let room = match res {
            None => {
                let msg = format!(
                    "No room named {} is registered, but you can create one!",
                    t.room_name
                );
                let res = JoinRoommPersistenceResponse::Failure(msg);
                t.tx.send(res).ok();
                return Ok(());
            }
            Some(r) => r,
        };

        if room.user_ids.iter().any(|bson_id| {
            bson_to_uuid(bson_id)
                // .ok_or(anyhow!("expected uuid value"))
                .unwrap()
                == t.user.id
        }) {
            let msg = format!("User {} already is in the room.", t.user.username);
            let res = JoinRoommPersistenceResponse::Failure(msg);
            t.tx.send(res).ok();
            return Ok(());
        }

        match (&room.pwd, &t.room_pwd) {
            (Some(correct_pwd), Some(provided_pwd)) => {
                if correct_pwd != provided_pwd {
                    let msg = format!("Incorrect room password.");
                    let res = JoinRoommPersistenceResponse::Failure(msg);
                    t.tx.send(res).ok();
                    return Ok(());
                }
            }
            (Some(_), None) => {
                let msg = format!("Room password required.");
                let res = JoinRoommPersistenceResponse::Failure(msg);
                t.tx.send(res).ok();
                return Ok(());
            }
            _ => {}
        }

        let mut users_cursor = users_collection
            .find(doc! { "id": { "$in": room.user_ids } })
            .await?;

        let mut users = vec![];

        while let Some(user_res) = users_cursor.next().await {
            let user = user_res?;

            users.push(User {
                id: bson_to_uuid(&user.id).ok_or(anyhow!("expected uuid value"))?,
                username: user.username,
            })
        }

        let data = RoomData {
            id: bson_to_uuid(&room.id).ok_or(anyhow!("expected uuid value"))?,
            name: t.room_name,
            users,
            users_online: vec![],
        };

        let filter = doc! { "username": &t.user.username };
        let update = doc! { "$push": { "room_ids": room.id } };

        users_collection.find_one_and_update(filter, update).await?;

        // user.rooms.push(room.id);

        let res = JoinRoommPersistenceResponse::Success(data);
        t.tx.send(res).ok();

        Ok(())
    }

    async fn handle_auth(t: AuthTransit, users_collection: Collection<DbUser>) -> Result<()> {
        let err_msg = String::from("Internal server error");
        let err_res = AuthResponse::Failure(err_msg);

        let filter = doc! { "username": &t.data.username };
        let user_res = users_collection.find_one(filter).await?;

        let db_user = match user_res {
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
                return Ok(());
            }
        };

        let parsed_hash = match PasswordHash::new(&db_user.pwd) {
            Ok(hash) => hash,
            Err(err) => {
                error!("error hashing password: {err}");
                if let Err(err) = t.tx.send(err_res) {
                    debug!("oneshot register res receiver dropped{err:?} {}", Bt::new());
                };
                return Ok(());
            }
        };

        let argon2 = Argon2::default();
        let res = match argon2.verify_password(t.data.pwd.as_bytes(), &parsed_hash) {
            Err(argon2::password_hash::Error::Password) => {
                AuthResponse::Failure(format!("Incorrect password"))
            }
            Err(err) => {
                error!("error hashing password: {err}");
                AuthResponse::Failure(format!("Internal server error"))
            }
            Ok(_) => AuthResponse::Success(User {
                username: t.data.username,
                id: bson_to_uuid(&db_user.id).ok_or(anyhow!("expected uuid value"))?,
            }),
        };

        if let Err(err) = t.tx.send(res) {
            debug!(
                "oneshot auth receiver dropped before auth finished {err:?} {}",
                Bt::new()
            )
        }
        Ok(())
    }

    async fn get_user_data(
        t: UserDataTransit,
        users_collection: Collection<DbUser>,
        rooms_collection: Collection<DbRoom>,
    ) -> Result<()> {
        let filter = doc! { "username": t.user.username };
        let user_res = users_collection.find_one(filter).await?;

        let user = match user_res {
            Some(user) => user,
            None => return Ok(()),
        };

        let mut user_rooms = vec![];

        let filter = doc! { "id": { "$in": user.room_ids } };

        let mut rooms_cursor = rooms_collection.find(filter).await?;

        while let Some(room_res) = rooms_cursor.next().await {
            let room = room_res?;

            let mut users_cursor = users_collection
                .find(doc! { "id": { "$in": room.user_ids } })
                .await?;

            let mut users = vec![];

            while let Some(user_res) = users_cursor.next().await {
                let user = user_res?;
                users.push(User {
                    id: bson_to_uuid(&user.id).unwrap(),
                    username: user.username,
                });
            }

            let room_data = RoomData {
                id: bson_to_uuid(&room.id).ok_or(anyhow!("expected uuid value"))?,
                name: room.name.clone(),
                users,
                users_online: vec![],
            };
            user_rooms.push(room_data);
        }

        let data = UserInitData { rooms: user_rooms };
        if let Err(err) = t.tx.send(data) {
            debug!(
                "oneshot receiver for get_user_data dropped {err:?} {}",
                Bt::new()
            );
        };
        Ok(())
    }

    async fn handle_register(
        t: RegisterDataTransit,
        users_collection: Collection<DbUser>,
        rooms_collection: Collection<DbRoom>,
        pwd_re: Regex,
        username_re: Regex,
    ) -> Result<()> {
        let err_msg = String::from("Internal server error, user not created");
        let err_res = RegisterResponse::Failure(err_msg);

        if !username_re.is_match(&t.data.username) {
            let err_msg = String::from(USERNAME_ERROR_MSG);
            let err_res = RegisterResponse::Failure(err_msg);
            if let Err(err) = t.tx.send(err_res) {
                debug!("oneshot register res receiver dropped{err:?} {}", Bt::new());
            };
            return Ok(());
        };

        let filter = doc! { "username": &t.data.username };
        let res = users_collection.find_one(filter).await?;

        if res.is_some() {
            let res = RegisterResponse::Failure(String::from("Username already taken"));
            if let Err(err) = t.tx.send(res) {
                debug!("oneshot register res receiver dropped{err:?} {}", Bt::new());
            };
            return Ok(());
        }

        if !pwd_re.is_match(&t.data.pwd)
            || !&t.data.pwd.chars().any(|c| c.is_lowercase())
            || !&t.data.pwd.chars().any(|c| c.is_uppercase())
            || !&t.data.pwd.chars().any(|c| c.is_ascii_digit())
        {
            let res = RegisterResponse::Failure(String::from(PASSWORD_ERROR_MSG));
            if let Err(err) = t.tx.send(res) {
                debug!("oneshot register res receiver dropped{err:?} {}", Bt::new());
            };
            return Ok(());
        }

        let public_room_id = uuid_to_bson(Uuid::from_str(PUBLIC_ROOM_ID)?);

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = match argon2.hash_password(t.data.pwd.as_bytes(), &salt) {
            Ok(hash) => hash.to_string(),
            Err(err) => {
                error!("error hashing password: {err}");
                if let Err(err) = t.tx.send(err_res) {
                    debug!("oneshot register res receiver dropped{err:?} {}", Bt::new());
                };
                return Ok(());
            }
        };

        let new_db_user = DbUser {
            id: uuid_to_bson(Uuid::new_v4()),
            pwd: password_hash,
            username: t.data.username,
            room_ids: vec![public_room_id.clone()],
        };

        let new_user = User {
            id: bson_to_uuid(&new_db_user.id).ok_or(anyhow!("expected uuid value"))?,
            username: new_db_user.username.clone(),
        };

        let filter = doc! { "id": public_room_id };
        let update: Document = doc! { "$push": { "user_ids": new_db_user.id.clone() } };

        rooms_collection.update_one(filter, update).await?;

        let res = RegisterResponse::Success(new_user);
        users_collection.insert_one(new_db_user).await?;

        if let Err(err) = t.tx.send(res) {
            debug!("oneshot register res receiver dropped{err:?} {}", Bt::new());
        };

        Ok(())
    }

    async fn handle_user_joined_room(
        t: UserRoomData,
        users_collection: Collection<DbUser>,
        rooms_collection: Collection<DbRoom>,
    ) -> Result<()> {
        let user_bson_id = uuid_to_bson(t.user.id);
        let room_bson_id = uuid_to_bson(t.room_id);

        let filter = doc! { "id": room_bson_id.clone() };
        let update = doc! { "$push": { "user_ids": user_bson_id.clone() } };

        rooms_collection.find_one_and_update(filter, update).await?;

        let filter = doc! { "id": user_bson_id };
        let update = doc! { "$push": { "room_ids": room_bson_id } };

        users_collection.find_one_and_update(filter, update).await?;

        Ok(())
    }

    async fn handle_user_left_room(
        t: UserRoomData,
        users_collection: Collection<DbUser>,
        rooms_collection: Collection<DbRoom>,
    ) -> Result<()> {
        let user_bson_id = uuid_to_bson(t.user.id);
        let room_bson_id = uuid_to_bson(t.room_id);

        let filter = doc! { "id": room_bson_id.clone() };
        let update = doc! { "$pull": { "user_ids": user_bson_id.clone() } };

        rooms_collection.find_one_and_update(filter, update).await?;

        let filter = doc! { "id": user_bson_id };
        let update = doc! { "$pull": { "room_ids": room_bson_id } };

        users_collection.find_one_and_update(filter, update).await?;

        Ok(())
    }
}
