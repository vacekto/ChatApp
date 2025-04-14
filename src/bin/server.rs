use chat_app::server_lib::{
    client_task::ClientTask,
    functions::{create_manager_task, init_client},
    util::{
        config::{log, CLIENT_MANAGER_CAPACITY, ROOM_CAPACITY, SERVER_HOSTNAME, SERVER_PORT},
        types::ClientToManagerMessage,
    },
};
use chat_app::shared_lib::util_functions::get_addr;
use std::error::Error;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc},
    task,
};
use tokio_util::bytes::Bytes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = get_addr(SERVER_HOSTNAME, SERVER_PORT);

    let listener = TcpListener::bind(&addr)
        .await
        .expect("Tcp listerner failed");

    println!("listening on: {}", addr);

    let (tx_public_room, _) = broadcast::channel::<Bytes>(ROOM_CAPACITY);

    let (tx_client_manager, rx_client_manager) =
        mpsc::channel::<ClientToManagerMessage>(CLIENT_MANAGER_CAPACITY);

    task::spawn(create_manager_task(rx_client_manager));

    loop {
        match listener.accept().await {
            Ok((tcp, _)) => {
                let tx_public_room = tx_public_room.clone();
                let tx_client_manager = tx_client_manager.clone();

                task::spawn(handle_connection(tcp, tx_client_manager, tx_public_room));
            }
            Err(err) => log(err.into(), None),
        }
    }
}

async fn handle_connection(
    mut tcp: TcpStream,
    tx_client_manager: mpsc::Sender<ClientToManagerMessage>,
    tx_public_room: broadcast::Sender<Bytes>,
) {
    // TODO: extend to fetch user data
    let client_data = match init_client(&mut tcp).await {
        Ok(init) => init,
        Err(err) => {
            log(err.into(), Some("failed client initialization"));
            return;
        }
    };

    let client = ClientTask::new(tcp, tx_client_manager, tx_public_room, client_data.id).await;

    client.run().await
}
