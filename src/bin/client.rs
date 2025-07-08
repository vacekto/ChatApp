use anyhow::Result;
use chat_app::{
    client_lib::{
        data_stream::handle_file_stream, read_server::listen_for_server, tui::tui,
        write_to_server::write_to_server,
    },
    shared_lib::types::{Chunk, ClientServerConnectMsg, ClientServerMsg},
};
use dotenv::dotenv;
use futures::StreamExt;
use rustls::{pki_types::CertificateDer, ClientConfig};
use std::sync::Arc;
use tokio_tungstenite::{connect_async_tls_with_config, tungstenite::protocol::WebSocketConfig};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let cert = std::fs::read(std::path::Path::new("cert.der"))?;
    let cert: CertificateDer<'_> = CertificateDer::from(cert);

    let mut root_store = rustls::RootCertStore::empty();
    root_store.add(cert)?;

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = tokio_tungstenite::Connector::Rustls(Arc::new(config.clone()));
    let ws_config = WebSocketConfig::default();

    let addr = format!("wss://localhost:{}", std::env::var("SERVER_PORT")?);

    let (wss, _) =
        connect_async_tls_with_config(&addr, Some(ws_config), false, Some(connector)).await?;

    let (wss_write, wss_read) = wss.split();
    let (tx_tcp_tui, rx_tcp_tui) = tokio::sync::mpsc::channel(20);
    let (tx_tcp_stream, rx_tcp_stream) = tokio::sync::mpsc::channel(20);
    let (tx_tui_tcp_file, rx_tui_tcp_file) = tokio::sync::mpsc::channel::<Chunk>(1000);
    let (tx_tui_tcp_msg, rx_tui_tcp_msg) = tokio::sync::mpsc::channel::<ClientServerMsg>(20);
    let (tx_tui_tcp_auth, rx_tui_tcp_auth) =
        tokio::sync::mpsc::channel::<ClientServerConnectMsg>(20);

    tokio::spawn(async move {
        handle_file_stream(rx_tcp_stream).await.ok();
    });

    tokio::spawn(async move {
        listen_for_server(wss_read, tx_tcp_tui, tx_tcp_stream)
            .await
            .ok();
    });

    tokio::spawn(async move {
        write_to_server(wss_write, rx_tui_tcp_msg, rx_tui_tcp_file, rx_tui_tcp_auth)
            .await
            .unwrap();
    });

    tui(rx_tcp_tui, tx_tui_tcp_file, tx_tui_tcp_msg, tx_tui_tcp_auth)
        .await
        .ok();

    Ok(())
}
