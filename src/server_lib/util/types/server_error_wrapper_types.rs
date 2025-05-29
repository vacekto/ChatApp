use thiserror::Error;

use super::server_error_types::{BincodeErr, TcpErr};

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
    DataParsing(#[from] TcpDataParsingError),
    #[error("{0}")]
    Unexpected(String),
}

#[derive(Error, Debug)]
pub enum AuthError {
    #[error(transparent)]
    DataParsing(#[from] TcpDataParsingError),
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
pub enum TcpDataParsingError {
    #[error(transparent)]
    Bincode(#[from] BincodeErr),
    #[error(transparent)]
    Tcp(#[from] TcpErr),
    #[error("Connection closed")]
    ConnectionClosed,
}
