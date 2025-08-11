use anyhow::Result;
use client::{
    data_stream::handle_file_stream, read_server::listen_for_server, tui,
    write_server::write_to_server,
};
use dotenv::dotenv;
use futures::StreamExt;
use shared::types::{Chunk, ClientServerAuthMsg, ClientServerMsg};
use tokio_tungstenite::connect_async;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let server_host = std::env::var("SERVER_HOST")?;
    let server_port = std::env::var("SERVER_PORT")?;

    let server_addr = format!("ws://{}:{}/server", server_host, server_port);

    let (ws, _) = connect_async(server_addr).await.expect("Failed to connect");
    let (ws_write, ws_read) = ws.split();

    let (tx_ws_tui, rx_ws_tui) = tokio::sync::mpsc::channel(20);
    let (tx_ws_stream, rx_ws_stream) = tokio::sync::mpsc::channel(20);
    let (tx_tui_ws_file, rx_tui_ws_file) = tokio::sync::mpsc::channel::<Chunk>(1000);
    let (tx_tui_ws_msg, rx_tui_ws_msg) = tokio::sync::mpsc::channel::<ClientServerMsg>(20);
    let (tx_tui_ws_auth, rx_tui_ws_auth) = tokio::sync::mpsc::channel::<ClientServerAuthMsg>(20);

    tokio::spawn(async move {
        handle_file_stream(rx_ws_stream).await.ok();
    });

    tokio::spawn(async move {
        listen_for_server(ws_read, tx_ws_tui, tx_ws_stream)
            .await
            .ok();
    });

    tokio::spawn(async move {
        write_to_server(ws_write, rx_tui_ws_msg, rx_tui_ws_file, rx_tui_ws_auth)
            .await
            .unwrap();
    });

    tui::app(rx_ws_tui, tx_tui_ws_file, tx_tui_ws_msg, tx_tui_ws_auth)
        .await
        .ok();

    Ok(())
}
