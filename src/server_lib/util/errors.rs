use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataParsingError {
    #[error("Failed to read or write framed message via TCP stream, actual error: {0}")]
    TcpReadWrite(#[from] std::io::Error),
    #[error("Failed to serialize / deserialize using bincode, actual error: {0}")]
    Bincode(#[from] Box<bincode::ErrorKind>),
}
