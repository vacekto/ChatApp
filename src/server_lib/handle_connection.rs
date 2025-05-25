use super::util::{
    server_functions::{
        authenticate, fetch_user_data, handle_register, read_client_data, send_server_msg,
    },
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

        let (user, init_server_data) = match client_msg {
            ClientServerConnectMsg::Register(register_data) => {
                let res =
                    handle_register(register_data, &tx_client_persistence, &tx_client_manager)
                        .await?;

                let msg = ServerClientMsg::Register(res);
                send_server_msg(&msg, &mut tcp_write).await?;
                continue;
            }
            ClientServerConnectMsg::Login(auth_data) => {
                let res = authenticate(auth_data, &tx_client_persistence).await?;

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

                let (init_server_data, init_client_data) =
                    fetch_user_data(user.clone(), &tx_client_persistence, &tx_client_manager)
                        .await?;

                let msg = ServerClientMsg::Init(init_client_data);
                send_server_msg(&msg, &mut tcp_write).await?;

                (user, init_server_data)
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

        let res = client.run(init_server_data).await;

        match res {
            ClientTaskResult::Close => return Ok(()),
            ClientTaskResult::Logout => continue,
        }
    }
}
