use chat_app::{
    server_lib::{
        client_task::ClientTask,
        manager_task::spawn_manager_task,
        persistence_task::spawn_persistence_task,
        util::{
            config::{CLIENT_MANAGER_CAPACITY, CLIENT_PERSISTENCE_CAPACITY},
            errors::AuthError,
            server_functions::{authenticate, get_location},
            types::{ClientManagerMsg, ClientPersistenceMsg, ClientTaskResult},
        },
    },
    shared_lib::{
        config::SERVER_ADDR,
        types::{AuthData, AuthResponse, ServerClientMsg},
    },
};
use futures::{SinkExt, StreamExt};
use log::{error, info, warn};
use std::error::Error;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
    task,
};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let listener = TcpListener::bind(SERVER_ADDR)
        .await
        .expect("Tcp listerner failed");

    info!("listening on: {}", SERVER_ADDR);

    let (tx_client_manager, rx_client_manager) =
        mpsc::channel::<ClientManagerMsg>(CLIENT_MANAGER_CAPACITY);

    let (tx_client_persistence, rx_client_persistence) =
        mpsc::channel::<ClientPersistenceMsg>(CLIENT_PERSISTENCE_CAPACITY);

    spawn_manager_task(rx_client_manager);
    spawn_persistence_task(rx_client_persistence);

    loop {
        match listener.accept().await {
            Ok((tcp, _)) => {
                let tx_client_manager = tx_client_manager.clone();
                let tx_client_persistence = tx_client_persistence.clone();

                task::spawn(async move {
                    handle_connection(tcp, tx_client_manager, tx_client_persistence).await;
                });
            }
            Err(err) => error!("{}, {}", err, get_location()),
        }
    }
}

async fn handle_connection(
    tcp: TcpStream,
    tx_client_manager: mpsc::Sender<ClientManagerMsg>,
    tx_client_persistence: mpsc::Sender<ClientPersistenceMsg>,
) {
    let (tcp_read, tcp_write) = tcp.into_split();
    let mut tcp_read = FramedRead::new(tcp_read, LengthDelimitedCodec::new());
    let mut tcp_write = FramedWrite::new(tcp_write, LengthDelimitedCodec::new());

    loop {
        let auth_bytes = match tcp_read.next().await {
            Some(r) => match r {
                Ok(b) => b,
                Err(err) => {
                    error!("{},  {}", err, get_location());
                    return;
                }
            },
            None => return,
        };

        let auth_data: AuthData = match bincode::deserialize(&auth_bytes)
            .map_err(|err| AuthError::DataParsing(err.into()))
        {
            Ok(d) => d,
            Err(err) => {
                warn!("error reading auth data, {}", err);
                return;
            }
        };

        let init_data = match authenticate(auth_data, &tx_client_manager).await {
            Ok(data) => data,
            Err(err) => match err {
                AuthError::UsernameTaken(username) => {
                    let failure_msg = format!("Username {} is already taken", username);
                    let res = AuthResponse::Failure(failure_msg);
                    let msg = ServerClientMsg::Auth(res);

                    let res_bytes = match bincode::serialize(&msg) {
                        Ok(b) => b,
                        Err(err) => {
                            error!("{}, {}", err, get_location());
                            return;
                        }
                    };

                    if let Err(err) = tcp_write.send(res_bytes.into()).await {
                        error!("{}", err);
                        return;
                    };
                    continue;
                }
                _ => {
                    error!("{}", err);
                    let failure_msg = String::from("Internal server error");
                    let res = AuthResponse::Failure(failure_msg);

                    let res_bytes = match bincode::serialize(&res) {
                        Ok(b) => b,
                        Err(err) => {
                            error!("{}, {}", err, get_location());
                            return;
                        }
                    };

                    if let Err(err) = tcp_write.send(res_bytes.into()).await {
                        error!("{}, {}", err, get_location());
                        return;
                    };

                    return;
                }
            },
        };

        let res = AuthResponse::Success(init_data.clone());
        let msg = ServerClientMsg::Auth(res);
        let res_bytes = match bincode::serialize(&msg) {
            Ok(b) => b,
            Err(err) => {
                error!("{}, {}", err, get_location());
                return;
            }
        };

        if let Err(err) = tcp_write.send(res_bytes.into()).await {
            error!("{}, {}", err, get_location());
            return;
        };

        let client: ClientTask = ClientTask::new(
            init_data,
            &mut tcp_read,
            &mut tcp_write,
            tx_client_manager.clone(),
            tx_client_persistence.clone(),
        )
        .await;

        let res = client.run().await;

        match res {
            ClientTaskResult::Close => return,
            ClientTaskResult::Logout => continue,
        }
    }
}
