use bytes::Bytes;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum DataProcessingError {
    #[error("Failed to read/write framed message vie TCP stream, actual error: {0}")]
    FramedTextMessage(#[from] std::io::Error),
    #[error("Failed to serialize / deserialize using bincode, actual error: {0}")]
    Bincode(#[from] Box<bincode::ErrorKind>),
}

#[derive(Error, Debug)]
pub enum ChannelNotFoundError {
    #[error("Direct channel with id {0} not found, shopuld attempt to find view manager")]
    Direct(Uuid, Bytes),
    #[error("Room channel with id {0} not found, shopuld attempt to find view manager")]
    Room(Uuid, Bytes),
}

#[derive(Error, Debug)]
#[error("Message transit error")]
pub enum MessageTransitError {
    Recoverable(#[from] ChannelNotFoundError),
    Unrecoverable(#[from] DataProcessingError),
}
