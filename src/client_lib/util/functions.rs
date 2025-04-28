use std::{io::Write, net::TcpStream};

use anyhow::Result;

use crate::{
    client_lib::{read_server::read_framed_tcp_msg, write_server::frame_data},
    shared_lib::types::{AuthData, InitClientData},
};

pub fn handle_auth(mut tcp: TcpStream, username: String) -> Result<InitClientData> {
    let auth = AuthData { username };
    let serialized = bincode::serialize(&auth)?;
    let framed = frame_data(&serialized);

    tcp.write_all(&framed)?;

    let bytes = read_framed_tcp_msg(&mut tcp)?;
    let init_data: InitClientData = bincode::deserialize(&bytes)?;

    Ok(init_data)
}
