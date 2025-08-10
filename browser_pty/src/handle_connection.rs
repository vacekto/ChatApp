#[derive(Debug, Error)]
enum HandlerError {
    #[error("Connection error: {0}")]
    Connection(#[from] warp::Error),

    #[error("expected text message: '{{x}} {{y}}' as dimensions")]
    InvalidDimentionsMessage,

    #[error("PTY error: {0}")]
    Pty(#[from] std::io::Error),

    #[error("Unexpected error: {0}")]
    Other(#[from] anyhow::Error),
}

fn parse_initial_msg(msg: Message) -> Result<(u16, u16), HandlerError> {
    let data = msg
        .to_str()
        .map_err(|_| HandlerError::InvalidDimentionsMessage)?;

    let mut str_data = data.split_whitespace();
    let x: u16 = str_data
        .next()
        .ok_or(HandlerError::InvalidDimentionsMessage)?
        .parse()
        .map_err(|_| HandlerError::InvalidDimentionsMessage)?;

    let y: u16 = str_data
        .next()
        .ok_or(HandlerError::InvalidDimentionsMessage)?
        .parse()
        .map_err(|_| HandlerError::InvalidDimentionsMessage)?;
    Ok((x, y))
}
