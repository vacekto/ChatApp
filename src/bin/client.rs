use anyhow::Result;
use chat_app::{
    client_lib::{
        data_stream::handle_file_stream, read_server::listen_for_server, tui::tui,
        write_to_server::write_to_server,
    },
    shared_lib::types::{Chunk, ClientServerConnectMsg, ClientServerMsg},
};
use rustls::{
    pki_types::{CertificateDer, ServerName},
    ClientConfig,
};
use std::{sync::Arc, thread, time::Duration};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let server_addr = match std::env::var("SERVER_PORT") {
        Ok(port) => format!("localhost:{port}"),
        Err(_) => String::from("localhost:11111"),
    };

    let tcp = loop {
        println!("attempting to establish connection../");
        match TcpStream::connect(&server_addr).await {
            Ok(s) => {
                println!("connection established with: :{}", server_addr);
                break s;
            }
            Err(err) => {
                println!("connection error: {}", err);
                thread::sleep(Duration::from_secs(3));
            }
        }
    };

    let cert = std::fs::read(std::path::Path::new("cert.der"))?;
    let cert = CertificateDer::from(cert);

    let mut root_store = rustls::RootCertStore::empty();
    root_store.add(cert)?;

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));
    let domain = ServerName::try_from("localhost")?;
    let tls_stream = connector.connect(domain, tcp).await?;

    let (reader, writer) = tokio::io::split(tls_stream);

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
        listen_for_server(reader, tx_tcp_tui, tx_tcp_stream)
            .await
            .ok();
    });

    tokio::spawn(async move {
        write_to_server(writer, rx_tui_tcp_msg, rx_tui_tcp_file, rx_tui_tcp_auth)
            .await
            .unwrap();
    });

    tui(rx_tcp_tui, tx_tui_tcp_file, tx_tui_tcp_msg, tx_tui_tcp_auth)
        .await
        .ok();

    Ok(())
}
