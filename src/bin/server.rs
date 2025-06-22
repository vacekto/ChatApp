use chat_app::server_lib::{
    handle_connection::handle_connection,
    manager_task::spawn_manager_task,
    persistence_task::spawn_persistence_task,
    util::{
        config::{CLIENT_MANAGER_CAPACITY, CLIENT_PERSISTENCE_CAPACITY},
        server_functions::load_cert,
        types::{
            server_data_types::{ClientManagerMsg, ClientPersistenceMsg},
            server_error_types::Bt,
        },
    },
};
use log::{error, info};
use std::{error::Error, sync::Arc};
use tokio::{net::TcpListener, sync::mpsc, task};
use tokio_rustls::TlsAcceptor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let (cert, key) = load_cert()?;
    let tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)?;

    let server_addr = match std::env::var("SERVER_PORT") {
        Ok(port) => format!("0.0.0.0:{port}"),
        Err(_) => String::from("0.0.0.0:11111"),
    };

    let acceptor = TlsAcceptor::from(Arc::new(tls_config));
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
                let acceptor = acceptor.clone();
                let tls = acceptor.accept(tcp).await?;
                let tx_client_manager = tx_client_manager.clone();
                let tx_client_persistence = tx_client_persistence.clone();

                task::spawn(async move {
                    if let Err(err) =
                        handle_connection(tls, tx_client_manager, tx_client_persistence).await
                    {
                        error!("closing connection due to: {err}");
                    };
                });
            }
            Err(err) => error!("Error establishing connection: {}, {}", err, Bt::new()),
        }
    }
}
