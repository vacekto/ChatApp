use dotenv::dotenv;
use log::{error, info};
use server::{
    handle_connection::handle_connection,
    manager_task::spawn_manager_task,
    persistence_task::spawn_persistence_task,
    util::{
        config::{CLIENT_MANAGER_CAPACITY, CLIENT_PERSISTENCE_CAPACITY},
        types::server_data_types::{ClientManagerMsg, ClientPersistenceMsg},
    },
};
use std::{env::var, error::Error};
use tokio::sync::mpsc;
use warp::Filter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let port: u16 = var("SERVER_PORT")?.parse()?;

    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("listening on: 0.0.0.0:{}", port);

    let (tx_client_manager, rx_client_manager) =
        mpsc::channel::<ClientManagerMsg>(CLIENT_MANAGER_CAPACITY);

    let _t = tx_client_manager.clone();

    let (tx_client_persistence, rx_client_persistence) =
        mpsc::channel::<ClientPersistenceMsg>(CLIENT_PERSISTENCE_CAPACITY);

    spawn_manager_task(rx_client_manager);
    spawn_persistence_task(rx_client_persistence);

    let tx_cm_filter = warp::any().map(move || tx_client_manager.clone());
    let tx_cp_filter = warp::any().map(move || tx_client_persistence.clone());

    let http_route = warp::path("health").map(|| "OK");

    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(tx_cm_filter)
        .and(tx_cp_filter)
        .map(|ws: warp::ws::Ws, tx_cm, tx_cp| {
            ws.on_upgrade(move |ws| async move {
                if let Err(err) = handle_connection(ws, tx_cm, tx_cp).await {
                    error!("closing connection due to: {err}");
                }
            })
        });

    let routes = ws_route.or(http_route);

    warp::serve(routes).run(([0, 0, 0, 0], port)).await;

    Ok(())
}
