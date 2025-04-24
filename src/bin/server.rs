use chat_app::{
    server_lib::{
        client_task::ClientTask,
        manager_task::create_manager_task,
        util::{
            config::{log, CLIENT_MANAGER_CAPACITY, ROOM_CAPACITY},
            types::ClientToManagerMessage,
        },
    },
    shared_lib::config::SERVER_ADDR,
};
use std::error::Error;
use tokio::{
    net::TcpListener,
    sync::{broadcast, mpsc},
    task,
};
use tokio_util::bytes::Bytes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(SERVER_ADDR)
        .await
        .expect("Tcp listerner failed");

    println!("listening on: {}", SERVER_ADDR);

    let (tx_public_room, _) = broadcast::channel::<Bytes>(ROOM_CAPACITY);

    let (tx_client_manager, rx_client_manager) =
        mpsc::channel::<ClientToManagerMessage>(CLIENT_MANAGER_CAPACITY);

    task::spawn(create_manager_task(rx_client_manager));

    loop {
        match listener.accept().await {
            Ok((tcp, _)) => {
                let tx_public_room = tx_public_room.clone();
                let tx_client_manager = tx_client_manager.clone();

                task::spawn(async {
                    let client: ClientTask =
                        ClientTask::new(tcp, tx_client_manager, tx_public_room).await;
                    client.run().await
                });
            }
            Err(err) => log(err.into(), None),
        }
    }
}
