use chat_app::server_lib::{
    handle_connection::handle_connection,
    manager_task::spawn_manager_task,
    persistence_task::spawn_persistence_task,
    util::{
        config::{CLIENT_MANAGER_CAPACITY, CLIENT_PERSISTENCE_CAPACITY},
        server_functions::load_cert,
        types::server_data_types::{ClientManagerMsg, ClientPersistenceMsg},
    },
};
use dotenv::dotenv;
use log::{error, info};
use std::{env, error::Error, sync::Arc};
use tokio::{net::TcpListener, sync::mpsc, task};
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::accept_async;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let (cert, key) = load_cert()?;
    let tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)?;

    let server_addr = format!("{}:{}", env::var("SERVER_HOST")?, env::var("SERVER_PORT")?);

    let listener = TcpListener::bind(&server_addr)
        .await
        .expect("Tcp listerner failed");

    let acceptor = TlsAcceptor::from(Arc::new(tls_config));
    let (tx_client_manager, rx_client_manager) =
        mpsc::channel::<ClientManagerMsg>(CLIENT_MANAGER_CAPACITY);

    let (tx_client_persistence, rx_client_persistence) =
        mpsc::channel::<ClientPersistenceMsg>(CLIENT_PERSISTENCE_CAPACITY);

    spawn_manager_task(rx_client_manager);
    spawn_persistence_task(rx_client_persistence);

    info!("server running on {}", server_addr);

    loop {
        if let Ok((tcp, _addr)) = listener.accept().await {
            let acceptor = acceptor.clone();
            let tls = acceptor.accept(tcp).await.unwrap();
            let wss = accept_async(tls).await.expect("WebSocket handshake failed");
            let tx_client_manager = tx_client_manager.clone();
            let tx_client_persistence = tx_client_persistence.clone();
            // let (write, mut read) = wss.split();

            task::spawn(async move {
                if let Err(err) =
                    handle_connection(wss, tx_client_manager, tx_client_persistence).await
                {
                    error!("closing connection due to: {err}");
                };
            });
        }
    }
}
