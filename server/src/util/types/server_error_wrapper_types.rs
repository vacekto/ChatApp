use super::server_error_types::{BincodeErr, WsErr};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientInitError {
    #[error(transparent)]
    Bincode(#[from] BincodeErr),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Error, Debug)]
pub enum AuthError {
    #[error(transparent)]
    DataParsing(#[from] WsDataParsingError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Error, Debug)]
pub enum LoginError {
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Error, Debug)]
pub enum WsDataParsingError {
    #[error(transparent)]
    Bincode(#[from] BincodeErr),
    #[error(transparent)]
    Wcp(#[from] WsErr),
    #[error("Connection closed")]
    ConnectionClosed,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
