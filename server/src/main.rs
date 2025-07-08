// use chat_app::server_lib::{
//     handle_connection::handle_connection,
//     manager_task::spawn_manager_task,
//     persistence_task::spawn_persistence_task,
//     util::{
//         config::{CLIENT_MANAGER_CAPACITY, CLIENT_PERSISTENCE_CAPACITY},
//         types::{
//             server_data_types::{ClientManagerMsg, ClientPersistenceMsg},
//             server_error_types::Bt,
//         },
//     },
// };
use dotenv::dotenv;
use log::{error, info};
use server::{
    handle_connection::handle_connection,
    manager_task::spawn_manager_task,
    persistence_task::spawn_persistence_task,
    util::{
        config::{CLIENT_MANAGER_CAPACITY, CLIENT_PERSISTENCE_CAPACITY},
        types::{
            server_data_types::{ClientManagerMsg, ClientPersistenceMsg},
            server_error_types::Bt,
        },
    },
};
use std::{env::var, error::Error};
use tokio::{net::TcpListener, sync::mpsc, task};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    let server_addr = format!("0.0.0.0:{}", var("SERVER_PORT")?);

    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let listener = TcpListener::bind(&server_addr)
        .await
        .expect("Tcp listerner failed");

    info!("listening on: {}", server_addr);

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
                    if let Err(err) =
                        handle_connection(tcp, tx_client_manager, tx_client_persistence).await
                    {
                        error!("closing connection due to: {err}");
                    };
                });
            }
            Err(err) => error!("Error establishing connection: {}, {}", err, Bt::new()),
        }
    }
}
