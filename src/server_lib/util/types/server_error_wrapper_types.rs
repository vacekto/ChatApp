use thiserror::Error;

use super::server_error_types::{BincodeErr, WsErr};

// #[derive(Error, Debug)]
// pub enum DataParsingErrorOriginal {
//     #[error("{0:?}")]
//     TcpReadWrite(#[from] std::io::Error),
//     #[error("{0:?}")]
//     Bincode(#[from] Box<bincode::ErrorKind>),
// }

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
    DataParsing(#[from] WssDataParsingError),
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
pub enum WssDataParsingError {
    #[error(transparent)]
    Bincode(#[from] BincodeErr),
    #[error(transparent)]
    Wss(#[from] WsErr),
    #[error("Connection closed")]
    ConnectionClosed,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
