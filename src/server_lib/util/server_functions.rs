use std::fs;

use super::types::{
    server_data_types::{
        AuthTransit, ClientManagerMsg, ClientPersistenceMsg, IsOnlineTransit, RegisterDataTransit,
    },
    server_error_types::{BincodeErr, Bt, TcpErr},
    server_error_wrapper_types::TcpDataParsingError,
};
use crate::{
    server_lib::util::types::server_data_types::{TlsRead, TlsWrite},
    shared_lib::types::{
        AuthData, AuthResponse, ClientServerConnectMsg, RegisterData, RegisterResponse,
        ServerClientMsg,
    },
};
use anyhow::{anyhow, Result};
use futures::{SinkExt, StreamExt};
use mongodb::bson::{spec::BinarySubtype, Binary, Bson};
use rustls::pki_types::PrivatePkcs8KeyDer;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

pub async fn send_server_msg<'a>(
    msg: &ServerClientMsg,
    tcp_write: &'a mut TlsWrite,
) -> Result<(), TcpDataParsingError> {
    let serialized = bincode::serialize(msg).map_err(|err| BincodeErr(err, Bt::new()))?;
    tcp_write
        .send(serialized.into())
        .await
        .map_err(|err| TcpErr(err, Bt::new()))?;

    Ok(())
}

pub async fn read_client_data<'a>(
    tcp_read: &'a mut TlsRead,
) -> Result<ClientServerConnectMsg, TcpDataParsingError> {
    let auth_bytes = match tcp_read.next().await {
        Some(res) => res.map_err(|err| TcpErr(err, Bt::new()))?,
        None => Err(TcpDataParsingError::ConnectionClosed)?,
    };

    let auth_data: ClientServerConnectMsg =
        bincode::deserialize(&auth_bytes).map_err(|err| BincodeErr(err, Bt::new()))?;

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
        let res = AuthResponse::Failure(String::from("User is already logged in"));
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

    if let RegisterResponse::Success(user) = &res {
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

pub fn load_cert() -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>)> {
    let cert_path = "cert.der";
    let key_path = "key.der";

    let (cert_bytes, key_bytes) = (fs::read(cert_path)?, fs::read(key_path)?);

    let cert = CertificateDer::from(cert_bytes);
    let key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key_bytes));

    Ok((cert, key))
}
