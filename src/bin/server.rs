use chat_app::{
    server_lib::{
        client_task::ClientTask,
        manager_task::create_manager_task,
        util::{
            config::{log, CLIENT_MANAGER_CAPACITY},
            types::ClientManagerMsg,
        },
    },
    shared_lib::{
        config::SERVER_ADDR,
        types::{AuthData, InitClientData},
    },
};
use futures::{SinkExt, StreamExt};
use std::error::Error;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
    task,
};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(SERVER_ADDR)
        .await
        .expect("Tcp listerner failed");

    println!("listening on: {}", SERVER_ADDR);

    let (tx_client_manager, rx_client_manager) =
        mpsc::channel::<ClientManagerMsg>(CLIENT_MANAGER_CAPACITY);

    create_manager_task(rx_client_manager);

    loop {
        match listener.accept().await {
            Ok((tcp, _)) => {
                let tx_client_manager = tx_client_manager.clone();
                task::spawn(async move {
                    handle_connection(tcp, tx_client_manager).await;
                });
            }
            Err(err) => log(err.into(), None),
        }
    }
}

async fn handle_connection(tcp: TcpStream, tx_client_manager: mpsc::Sender<ClientManagerMsg>) {
    let (tcp_read, tcp_write) = tcp.into_split();
    let mut tcp_read = FramedRead::new(tcp_read, LengthDelimitedCodec::new());
    let mut tcp_write = FramedWrite::new(tcp_write, LengthDelimitedCodec::new());

    let auth_bytes = tcp_read.next().await.unwrap().unwrap();
    let auth_data: AuthData = bincode::deserialize(&auth_bytes).expect("incorrect auth data");

    let client_id = Uuid::new_v4();

    let init_data = InitClientData {
        id: client_id,
        username: auth_data.username,
    };

    let encoded = bincode::serialize(&init_data).unwrap();
    tcp_write.send(encoded.into()).await.unwrap();

    let tx_client_manager = tx_client_manager.clone();

    let client: ClientTask =
        ClientTask::new(init_data, tcp_read, tcp_write, tx_client_manager).await;
    client.run().await
}
