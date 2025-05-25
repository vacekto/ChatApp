use super::types::{
    server_data_types::{
        AuthTransit, ClientManagerMsg, ClientPersistenceMsg, GetConnectedUsersTransit,
        RegisterDataTransit, UserDataTransit, UserServerData,
    },
    server_error_types::{BincodeErr, Bt, TcpErr},
    server_error_wrapper_types::{DataParsingErrorOriginal, TcpDataParsingError},
};
use crate::shared_lib::types::{
    AuthData, AuthResponse, Chunk, ClientServerConnectMsg, FileMetadata, RegisterData,
    RegisterResponse, ServerClientMsg, TextMsg, User, UserClientData,
};
use anyhow::anyhow;
use backtrace::Backtrace;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use tokio::{
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    sync::{mpsc, oneshot},
};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

pub fn serialize_text_msg(msg: TextMsg) -> Result<Bytes, DataParsingErrorOriginal> {
    let msg = ServerClientMsg::Text(msg);
    let serialized = bincode::serialize(&msg)?;
    Ok(Bytes::from(serialized))
}

pub fn serialize_file_chunk(chunk: Chunk) -> Result<Bytes, DataParsingErrorOriginal> {
    let msg = ServerClientMsg::FileChunk(chunk);
    let serialized = bincode::serialize(&msg)?;
    Ok(Bytes::from(serialized))
}

pub fn serialize_file_metadata(data: FileMetadata) -> Result<Bytes, DataParsingErrorOriginal> {
    let msg = ServerClientMsg::FileMetadata(data);
    let serialized = bincode::serialize(&msg)?;
    Ok(Bytes::from(serialized))
}

// returns file and line in which this function is called without the whole backtrace, for debugging purpuses
pub fn get_location() -> String {
    let bt = Backtrace::new();

    let location = bt
        .frames()
        .iter()
        .skip(1)
        .flat_map(|frame| frame.symbols())
        .find_map(|symbol| {
            if let (Some(file), Some(line)) = (symbol.filename(), symbol.lineno()) {
                Some((file.display().to_string(), line))
            } else {
                None
            }
        });

    if let Some((file, line)) = location {
        format!("\nlocation: {file}:{line}")
    } else {
        format!("(location unknown)")
    }
}

pub async fn send_server_msg<'a>(
    msg: &ServerClientMsg,
    tcp_write: &'a mut FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
) -> Result<(), TcpDataParsingError> {
    let serialized = bincode::serialize(msg).map_err(|err| BincodeErr(err, Bt::new()))?;
    tcp_write
        .send(serialized.into())
        .await
        .map_err(|err| TcpErr(err, Bt::new()))?;

    Ok(())
}

pub async fn read_client_data<'a>(
    tcp_read: &'a mut FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
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
) -> Result<AuthResponse, anyhow::Error> {
    let (tx_ack, rx_ack) = oneshot::channel::<AuthResponse>();

    let transit = AuthTransit {
        data: auth_data,
        tx: tx_ack,
    };

    let manager_msg = ClientPersistenceMsg::Authenticate(transit);

    tx_client_persistence
        .send(manager_msg)
        .await
        .map_err(|err| anyhow!("{}{}", err, Bt::new()))?;

    Ok(rx_ack
        .await
        .map_err(|err| anyhow!("{}{}", err, Bt::new()))?)
}

pub async fn fetch_user_data(
    user: User,
    tx_client_persistence: &mpsc::Sender<ClientPersistenceMsg>,
    tx_client_manager: &mpsc::Sender<ClientManagerMsg>,
) -> Result<(UserServerData, UserClientData), anyhow::Error> {
    let (tx_ack, rx_ack) = oneshot::channel();

    let transit = UserDataTransit { tx: tx_ack, user };

    let msg = ClientPersistenceMsg::GetUserData(transit);

    tx_client_persistence
        .send(msg)
        .await
        .map_err(|err| anyhow!("tx_client_persistence dropped: {err} {}", get_location()))?;

    let init_server_data = rx_ack.await.map_err(|err| {
        anyhow!(
            "oneshot transmitter for client init got dropped: {err} {}",
            get_location()
        )
    })?;

    let (tx_ack, rx_ack) = oneshot::channel();

    let transit = GetConnectedUsersTransit {
        tx_ack,
        rooms: init_server_data.rooms.clone(),
    };
    let msg = ClientManagerMsg::GetConnectedUsers(transit);

    tx_client_manager
        .send(msg)
        .await
        .map_err(|err| anyhow!("{err}{}", Bt::new()))?;

    let tui_rooms = rx_ack.await.map_err(|err| anyhow!("{err}{}", Bt::new()))?;
    let init_client_data = UserClientData { rooms: tui_rooms };

    Ok((init_server_data, init_client_data))
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

pub async fn fetch_users_online() {}
