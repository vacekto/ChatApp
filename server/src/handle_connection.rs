use super::util::{
    server_functions::{authenticate, handle_register, read_client_data, send_server_msg},
    types::server_data_types::{ClientManagerMsg, ClientPersistenceMsg, ClientTaskResult},
};
use crate::{
    client_task::ClientTask,
    util::types::{server_data_types::Ws, server_error_wrapper_types::WsDataParsingError},
};
use anyhow::{Result, anyhow};
use futures::StreamExt;
use shared::types::{AuthResponse, ClientServerAuthMsg, ServerClientMsg};
use tokio::sync::mpsc;

pub async fn handle_connection<'a>(
    wss: Ws,
    tx_client_manager: mpsc::Sender<ClientManagerMsg>,
    tx_client_persistence: mpsc::Sender<ClientPersistenceMsg>,
) -> Result<()> {
    let (mut wss_write, mut wss_read) = wss.split();

    loop {
        let client_msg = match read_client_data(&mut wss_read).await {
            Ok(data) => data,
            Err(err) => match err {
                WsDataParsingError::ConnectionClosed => return Ok(()),
                _ => return Err(anyhow!(err)),
            },
        };

        let user = match client_msg {
            ClientServerAuthMsg::Register(register_data) => {
                let res =
                    handle_register(register_data, &tx_client_persistence, &tx_client_manager)
                        .await?;

                let msg = ServerClientMsg::Register(res);
                send_server_msg(&msg, &mut wss_write).await?;
                continue;
            }
            ClientServerAuthMsg::Login(auth_data) => {
                let res =
                    authenticate(auth_data, &tx_client_persistence, &tx_client_manager).await?;

                let user = match &res {
                    AuthResponse::Err(_) => {
                        let msg = ServerClientMsg::Auth(res);
                        send_server_msg(&msg, &mut wss_write).await?;
                        continue;
                    }
                    AuthResponse::Ok(user) => user.clone(),
                };

                let msg = ServerClientMsg::Auth(res);
                send_server_msg(&msg, &mut wss_write).await?;

                user
            }
        };

        let client: ClientTask = ClientTask::new(
            user,
            &mut wss_read,
            &mut wss_write,
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
