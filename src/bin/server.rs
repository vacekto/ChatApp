use chat_app::shared_lib::{get_addr, TextMessage};
use futures::{SinkExt, StreamExt};
use tokio::{
    net::{TcpListener, TcpStream},
    select,
    sync::broadcast::{self, Receiver, Sender},
    task,
};
use tokio_util::{
    bytes::Bytes,
    codec::{Framed, LengthDelimitedCodec},
};

const DEFUALT_HOSTNAME: &str = "localhost";
const DEFUALT_PORT: &str = "11111";

#[tokio::main]
async fn main() {
    let addr = get_addr(DEFUALT_HOSTNAME, DEFUALT_PORT);

    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("listening on: {}", addr);

    let (tx, _) = broadcast::channel::<TextMessage>(20);

    loop {
        let (tcp, _) = listener.accept().await.unwrap();
        let tx = tx.clone();
        let rx = tx.subscribe();

        task::spawn(handle_connection(tcp, tx, rx));
    }
}

async fn handle_connection(
    mut tcp: TcpStream,
    tx: Sender<TextMessage>,
    mut rx: Receiver<TextMessage>,
) {
    let mut framed = Framed::new(&mut tcp, LengthDelimitedCodec::new());

    loop {
        select! {
            result = framed.next() =>{
                if let Some(frame) = result {
                    let bytes = frame.unwrap();
                    let msg: TextMessage = bincode::deserialize(&bytes).unwrap();
                    println!("{:?}", msg);
                    tx.send(msg).unwrap();
                }
            }

            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        let s = bincode::serialize(&msg).unwrap();
                        framed.send(Bytes::from(s)).await.unwrap();
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        println!("Missed {} messages", n);
                    }
                    Err(_) => break,
                }
            }
        }
    }
}
