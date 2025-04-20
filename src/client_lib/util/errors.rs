use thiserror::Error;

#[derive(Error, Debug)]
pub enum ThreadError {
    #[error("Thread WriteServer returned with following error: {0}")]
    WriteServer(#[source] anyhow::Error),
    #[error("Thread ReadServer returned with following error: {0}")]
    ReadServer(#[source] anyhow::Error),
    #[error("Thread StdIn returned with following error: {0}")]
    StdIn(#[source] anyhow::Error),
}
