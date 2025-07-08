use crate::client_task::ClientTask;

use super::util::{
    server_functions::{authenticate, handle_register, read_client_data, send_server_msg},
    types::{
        server_data_types::{ClientManagerMsg, ClientPersistenceMsg, ClientTaskResult},
        server_error_wrapper_types::TcpDataParsingError,
    },
};
use anyhow::{Result, anyhow};
use shared::types::{ClientServerConnectMsg, ServerClientMsg};
use tokio::{net::TcpStream, sync::mpsc};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

pub async fn handle_connection<'a>(
    tcp: TcpStream,
    tx_client_manager: mpsc::Sender<ClientManagerMsg>,
    tx_client_persistence: mpsc::Sender<ClientPersistenceMsg>,
) -> Result<()> {
    let (tcp_read, tcp_write) = tcp.into_split();
    let mut tcp_read = FramedRead::new(tcp_read, LengthDelimitedCodec::new());
    let mut tcp_write = FramedWrite::new(tcp_write, LengthDelimitedCodec::new());

    loop {
        let client_msg = match read_client_data(&mut tcp_read).await {
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
                    Err(_) => {
                        let msg = ServerClientMsg::Auth(res);
                        send_server_msg(&msg, &mut tcp_write).await?;
                        continue;
                    }
                    Ok(user) => user.clone(),
                };

                let msg = ServerClientMsg::Auth(res);
                send_server_msg(&msg, &mut tcp_write).await?;

                user
            }
        };

        let client: ClientTask = ClientTask::new(
            user,
            &mut tcp_read,
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
