use super::types::{
    server_data_types::{
        AuthTransit, ClientManagerMsg, ClientPersistenceMsg, IsOnlineTransit, RegisterDataTransit,
    },
    server_error_types::{BincodeErr, Bt, WsErr},
    server_error_wrapper_types::WsDataParsingError,
};
use crate::util::types::server_data_types::{WsRead, WsWrite};
use anyhow::{Result, anyhow};
use bincode::deserialize;
use futures::{SinkExt, StreamExt};
use mongodb::bson::{Binary, Bson, spec::BinarySubtype};
use shared::types::{
    AuthData, AuthResponse, ClientServerAuthMsg, RegisterData, RegisterResponse, ServerClientMsg,
};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

pub async fn send_server_msg<'a>(
    msg: &ServerClientMsg,
    ws_write: &'a mut WsWrite,
) -> Result<(), WsDataParsingError> {
    let serialized = bincode::serialize(msg).map_err(|err| BincodeErr(err, Bt::new()))?;
    ws_write
        .send(serialized.into())
        .await
        .map_err(|err| WsErr(err, Bt::new()))?;

    Ok(())
}

pub async fn read_client_data<'a>(
    ws_read: &'a mut WsRead,
) -> Result<ClientServerAuthMsg, WsDataParsingError> {
    let ws_msg = match ws_read.next().await {
        Some(res) => res.map_err(|err| WsErr(err, Bt::new()))?,
        None => Err(WsDataParsingError::ConnectionClosed)?,
    };

    let auth_data = match ws_msg {
        Message::Binary(bytes) => deserialize(&bytes).map_err(|err| BincodeErr(err, Bt::new()))?,
        _ => unreachable!(),
    };

    Ok(auth_data)
}

pub async fn authenticate(
    auth_data: AuthData,
    tx_client_persistence: &mpsc::Sender<ClientPersistenceMsg>,
    tx_client_manager: &mpsc::Sender<ClientManagerMsg>,
) -> Result<AuthResponse, anyhow::Error> {
    let (tx_manager_ack, rx_manager_ack) = oneshot::channel::<bool>();
    let manager_transit = IsOnlineTransit {
        ack: tx_manager_ack,
        username: auth_data.username.clone(),
    };

    let manager_msg = ClientManagerMsg::IsOnline(manager_transit);

    tx_client_manager
        .send(manager_msg)
        .await
        .map_err(|err| anyhow!("{}{}", err, Bt::new()))?;

    let is_online = rx_manager_ack
        .await
        .map_err(|err| anyhow!("{}{}", err, Bt::new()))?;

    if is_online {
        let res = Err(String::from("User is already logged in"));
        return Ok(res);
    }

    let (tx_ack, rx_ack) = oneshot::channel::<AuthResponse>();

    let transit = AuthTransit {
        data: auth_data,
        tx: tx_ack,
    };

    let persistence_msg = ClientPersistenceMsg::Authenticate(transit);

    tx_client_persistence
        .send(persistence_msg)
        .await
        .map_err(|err| anyhow!("{}{}", err, Bt::new()))?;

    Ok(rx_ack
        .await
        .map_err(|err| anyhow!("{}{}", err, Bt::new()))?)
}

pub async fn handle_register(
    data: RegisterData,
    tx_client_persistence: &mpsc::Sender<ClientPersistenceMsg>,
    tx_client_manager: &mpsc::Sender<ClientManagerMsg>,
) -> Result<RegisterResponse, anyhow::Error> {
    let (tx_ack, rx_ack) = oneshot::channel();
    let transit = RegisterDataTransit {
        data: data,
        tx: tx_ack,
    };

    tx_client_persistence
        .send(ClientPersistenceMsg::Register(transit))
        .await
        .map_err(|err| anyhow!("rx_client_persistence dropped:  {err}  {}", Bt::new()))?;

    let res = rx_ack
        .await
        .map_err(|err| anyhow!("rx_client_persistence dropped:  {err}  {}", Bt::new()))?;

    if let Ok(user) = &res {
        let msg = ClientManagerMsg::UserRegistered(user.clone());
        tx_client_manager.send(msg).await?;
    };
    Ok(res)
}

pub fn uuid_to_bson(uuid: Uuid) -> Bson {
    Bson::Binary(Binary {
        subtype: BinarySubtype::Uuid,
        bytes: uuid.as_bytes().to_vec(),
    })
}

pub fn bson_to_uuid(bson: &Bson) -> Option<Uuid> {
    if let Bson::Binary(Binary {
        subtype: BinarySubtype::Uuid,
        bytes,
    }) = bson
    {
        Uuid::from_slice(bytes).ok()
    } else {
        None
    }
}
