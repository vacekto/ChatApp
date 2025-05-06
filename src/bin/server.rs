use bytes::BytesMut;
use chat_app::{
    server_lib::{
        client_task::ClientTask,
        manager_task::spawn_manager_task,
        util::{
            config::{log, CLIENT_MANAGER_CAPACITY},
            errors::AuthError,
            types::{ClientManagerMsg, ClientTaskResult, UsernameCheck},
        },
    },
    shared_lib::{
        config::SERVER_ADDR,
        types::{AuthData, AuthResponse, InitClientData, ServerTuiMsg},
    },
};
use futures::{SinkExt, StreamExt};
use std::error::Error;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
    task,
};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(SERVER_ADDR)
        .await
        .expect("Tcp listerner failed");

    println!("listening on: {}", SERVER_ADDR);

    let (tx_client_manager, rx_client_manager) =
        mpsc::channel::<ClientManagerMsg>(CLIENT_MANAGER_CAPACITY);

    spawn_manager_task(rx_client_manager);

    loop {
        match listener.accept().await {
            Ok((tcp, _)) => {
                let tx_client_manager = tx_client_manager.clone();
                task::spawn(async move {
                    handle_connection(tcp, tx_client_manager).await;
                });
            }
            Err(err) => log(err.into(), None),
        }
    }
}

async fn handle_connection(tcp: TcpStream, tx_client_manager: mpsc::Sender<ClientManagerMsg>) {
    let (tcp_read, tcp_write) = tcp.into_split();
    let mut tcp_read = FramedRead::new(tcp_read, LengthDelimitedCodec::new());
    let mut tcp_write = FramedWrite::new(tcp_write, LengthDelimitedCodec::new());

    loop {
        // let tcp_read = &mut tcp_read;
        let auth_bytes = match tcp_read.next().await {
            Some(r) => match r {
                Ok(b) => b,
                Err(err) => {
                    log(err.into(), None);
                    return;
                }
            },
            None => return, // client disconnected
        };

        let init_data = match authenticate(auth_bytes, &tx_client_manager).await {
            Ok(data) => data,
            Err(err) => match err {
                AuthError::UsernameTaken(username) => {
                    let failure_msg = format!("Username {} is already taken", username);
                    let res = AuthResponse::Failure(failure_msg);
                    let msg = ServerTuiMsg::Auth(res);

                    let res_bytes = match bincode::serialize(&msg) {
                        Ok(b) => b,
                        Err(err) => {
                            log(err.into(), None);
                            return;
                        }
                    };

                    if let Err(err) = tcp_write.send(res_bytes.into()).await {
                        log(err.into(), None);
                        return;
                    };
                    continue;
                }
                _ => {
                    log(err.into(), None);
                    let failure_msg = String::from("Internal server error");
                    let res = AuthResponse::Failure(failure_msg);

                    let res_bytes = match bincode::serialize(&res) {
                        Ok(b) => b,
                        Err(err) => {
                            log(err.into(), None);
                            return;
                        }
                    };

                    if let Err(err) = tcp_write.send(res_bytes.into()).await {
                        log(err.into(), None);
                        return;
                    };

                    return;
                }
            },
        };

        let res = AuthResponse::Success(init_data.clone());
        let msg = ServerTuiMsg::Auth(res);
        let res_bytes = match bincode::serialize(&msg) {
            Ok(b) => b,
            Err(err) => {
                log(err.into(), None);
                return;
            }
        };

        if let Err(err) = tcp_write.send(res_bytes.into()).await {
            log(err.into(), None);
            return;
        };
        let client: ClientTask = ClientTask::new(
            init_data,
            &mut tcp_read,
            &mut tcp_write,
            tx_client_manager.clone(),
        )
        .await;

        let res = client.run().await;

        match res {
            ClientTaskResult::Close => {
                println!("vypnutí");
                return;
            }
            ClientTaskResult::Logout => {
                println!("odhlášení");
                continue;
            }
        }
    }
}

async fn authenticate(
    auth_bytes: BytesMut,
    tx_client_manager: &mpsc::Sender<ClientManagerMsg>,
) -> Result<InitClientData, AuthError> {
    let auth_data: AuthData =
        bincode::deserialize(&auth_bytes).map_err(|err| AuthError::DataParsing(err.into()))?;

    let (tx_ack, rx_ack) = oneshot::channel::<bool>();
    let manager_msg = ClientManagerMsg::CheckUsername(UsernameCheck {
        username: auth_data.username.clone(),
        tx: tx_ack,
    });

    tx_client_manager
        .send(manager_msg)
        .await
        .map_err(|_| AuthError::Unexpected("rx_client_manager dropped!!".into()))?;

    let is_taken = rx_ack
        .await
        .map_err(|_| AuthError::Unexpected("Auth one shot channel transmitter dropped!!".into()))?;

    if is_taken {
        return Err(AuthError::UsernameTaken(auth_data.username));
    }

    let client_id = Uuid::new_v4();
    let init_data = InitClientData {
        id: client_id,
        username: auth_data.clone().username,
    };
    Ok(init_data)
}
