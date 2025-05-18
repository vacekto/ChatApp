use super::{
    errors::{AuthError, DataParsingError},
    types::{CheckUsernameTransit, ClientManagerMsg},
};
use crate::shared_lib::types::{
    AuthData, Chunk, FileMetadata, InitUserData, ServerClientMsg, TextMsg,
};
use backtrace::Backtrace;
use bytes::Bytes;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

pub fn serialize_text_msg(msg: TextMsg) -> Result<Bytes, DataParsingError> {
    let msg = ServerClientMsg::Text(msg);
    let serialized = bincode::serialize(&msg)?;
    Ok(Bytes::from(serialized))
}

pub fn serialize_file_chunk(chunk: Chunk) -> Result<Bytes, DataParsingError> {
    let msg = ServerClientMsg::FileChunk(chunk);
    let serialized = bincode::serialize(&msg)?;
    Ok(Bytes::from(serialized))
}

pub fn serialize_file_metadata(data: FileMetadata) -> Result<Bytes, DataParsingError> {
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

pub async fn authenticate(
    auth_data: AuthData,
    tx_client_manager: &mpsc::Sender<ClientManagerMsg>,
) -> Result<InitUserData, AuthError> {
    let (tx_ack, rx_ack) = oneshot::channel::<bool>();

    let manager_msg = ClientManagerMsg::CheckUsername(CheckUsernameTransit {
        username: auth_data.username.clone(),
        tx: tx_ack,
    });

    tx_client_manager
        .send(manager_msg)
        .await
        .map_err(|_| AuthError::Unrecoverable("rx_client_manager dropped!!".into()))?;

    let is_taken = rx_ack.await.map_err(|_| {
        AuthError::Unrecoverable("Auth one shot channel transmitter dropped!!".into())
    })?;

    if is_taken {
        return Err(AuthError::UsernameTaken(auth_data.username));
    }

    let client_id = Uuid::new_v4();
    let init_data = InitUserData {
        id: client_id,
        username: auth_data.clone().username,
    };
    Ok(init_data)
}
