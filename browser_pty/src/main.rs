use crate::util::{HandlerError, parse_initial_msg};
use anyhow::Result;
use dotenv::dotenv;
use futures::{SinkExt, StreamExt};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::env::var;
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use warp::Filter;
use warp::ws::{Message, WebSocket};
pub mod util;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let port: u16 = var("BROWSER_PTY_PORT")?.parse()?;

    let index = warp::path::end().and(warp::fs::file("static/index.html"));
    let static_files = warp::path("static").and(warp::fs::dir("static/"));

    let http_route = warp::path("health").map(|| "OK");

    let ws_route = warp::path("ws").and(warp::ws()).map(|ws: warp::ws::Ws| {
        ws.on_upgrade(|ws| async {
            if let Err(err) = handle_ws(ws).await {
                println!("{err}");
            }
        })
    });

    let routes = index.or(static_files).or(ws_route).or(http_route);
    println!("Server running on 0.0.0.0:{}", port);
    warp::serve(routes).run(([0, 0, 0, 0], port)).await;

    Ok(())
}

pub async fn handle_ws(ws: WebSocket) -> Result<(), HandlerError> {
    let (ws_tx, mut ws_rx) = ws.split();
    let (pty_out_tx, mut pty_out_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let (pty_in_tx, mut pty_in_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let ws_tx = Arc::new(Mutex::new(ws_tx));

    let msg = match ws_rx.next().await {
        None => return Ok(()),
        Some(data) => data?,
    };

    let (x, y) = parse_initial_msg(msg)?;

    let pty_system = NativePtySystem::default();
    let pair = pty_system.openpty(PtySize {
        rows: x,
        cols: y,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let cwd = std::env::current_dir()?;
    let cwd = cwd.to_str().unwrap();

    let mut child = pair
        .slave
        .spawn_command(CommandBuilder::new(format!("{cwd}/assets/client")))?;

    let mut pty_reader = pair.master.try_clone_reader()?;
    let mut pty_writer = pair.master.take_writer()?;

    tokio::task::spawn_blocking(move || {
        let mut buf = [0u8; 512];

        loop {
            let n = match pty_reader.read(&mut buf) {
                Ok(n) if n > 0 => n,
                _ => break,
            };

            if pty_out_tx.send(buf[..n].to_vec()).is_err() {
                break;
            }
        }
    });

    tokio::spawn(async move {
        while let Some(data) = pty_out_rx.recv().await {
            let mut ws_tx = ws_tx.lock().await;
            ws_tx.send(Message::binary(data)).await.ok();
        }
    });

    tokio::task::spawn_blocking(move || {
        while let Some(data) = pty_in_rx.blocking_recv() {
            if pty_writer.write_all(&data).is_err() {
                break;
            }
            let _ = pty_writer.flush();
        }
    });

    while let Some(Ok(msg)) = ws_rx.next().await {
        let data = if msg.is_binary() {
            msg.into_bytes()
        } else if let Ok(text) = msg.to_str() {
            text.as_bytes().to_vec()
        } else {
            continue;
        };

        if pty_in_tx.send(data).is_err() {
            break;
        }
    }
    drop(pty_in_tx);

    tokio::task::spawn_blocking(move || {
        child.kill().ok();
        child.wait().ok();
    });

    Ok(())
}
