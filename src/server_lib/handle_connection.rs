use super::util::{
    server_functions::{authenticate, handle_register, read_client_data, send_server_msg},
    types::{
        server_data_types::{ClientManagerMsg, ClientPersistenceMsg, ClientTaskResult},
        server_error_wrapper_types::TcpDataParsingError,
    },
};
use crate::{
    server_lib::client_task::ClientTask,
    shared_lib::types::{AuthResponse, ClientServerConnectMsg, ServerClientMsg},
};
use anyhow::{anyhow, Result};
use log::debug;
use tokio::{net::TcpStream, sync::mpsc};
use tokio_rustls::server::TlsStream;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

pub async fn handle_connection<'a>(
    tls_stream: TlsStream<TcpStream>,
    tx_client_manager: mpsc::Sender<ClientManagerMsg>,
    tx_client_persistence: mpsc::Sender<ClientPersistenceMsg>,
) -> Result<()> {
    let (tcp_read, tcp_write) = tokio::io::split(tls_stream);
    let mut tls_read = FramedRead::new(tcp_read, LengthDelimitedCodec::new());
    let mut tcp_write = FramedWrite::new(tcp_write, LengthDelimitedCodec::new());

    loop {
        debug!("3");
        let client_msg = match read_client_data(&mut tls_read).await {
            Ok(data) => data,
            Err(err) => match err {
                TcpDataParsingError::ConnectionClosed => return Ok(()),
                _ => return Err(anyhow!(err)),
            },
        };

        let user = match client_msg {
            ClientServerConnectMsg::Register(register_data) => {
                let res =
                    handle_register(register_data, &tx_client_persistence, &tx_client_manager)
                        .await?;

                let msg = ServerClientMsg::Register(res);
                send_server_msg(&msg, &mut tcp_write).await?;
                continue;
            }
            ClientServerConnectMsg::Login(auth_data) => {
                let res =
                    authenticate(auth_data, &tx_client_persistence, &tx_client_manager).await?;

                let user = match &res {
                    AuthResponse::Failure(_) => {
                        let msg = ServerClientMsg::Auth(res);
                        send_server_msg(&msg, &mut tcp_write).await?;
                        continue;
                    }
                    AuthResponse::Success(user) => user.clone(),
                };

                let msg = ServerClientMsg::Auth(res);
                send_server_msg(&msg, &mut tcp_write).await?;

                user
            }
        };

        let client: ClientTask = ClientTask::new(
            user,
            &mut tls_read,
            &mut tcp_write,
            tx_client_manager.clone(),
            tx_client_persistence.clone(),
        )
        .await;

        let res = client.run().await;

        match res {
            ClientTaskResult::Close => return Ok(()),
            ClientTaskResult::Logout => continue,
        }
    }
}
