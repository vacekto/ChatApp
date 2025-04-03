use std::{
    collections::HashMap,
    env,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread,
};
use uuid::Uuid;

const DEFUALT_HOSTNAME: &str = "localhost";
const DEFUALT_PORT: &str = "11111";

struct Client {
    id: Uuid,
    stream: TcpStream,
}

struct Message {
    from: Uuid,
    text: String,
}

enum Action {
    SendMessage(Message),
    TerminateConnection(Uuid),
}

fn main() {
    let mut args = env::args();

    let hostname = match args.nth(1) {
        Some(h) => h,
        None => String::from(DEFUALT_HOSTNAME),
    };

    let port = match args.nth(2) {
        Some(p) => p,
        None => String::from(DEFUALT_PORT),
    };

    let addr = (hostname + ":") + &port;
    let listener = TcpListener::bind(addr.to_string()).unwrap();
    println!("listening on: {}", addr);

    let clients = Arc::new(Mutex::new(HashMap::<Uuid, Client>::new()));

    let (tx, rx) = mpsc::channel();
    let also_clients = clients.clone();
    thread::spawn(move || {
        while let Ok(a) = rx.recv() {
            match a {
                Action::SendMessage(msg) => {
                    let mut guard = also_clients.lock().unwrap();
                    for (_, client) in guard.iter_mut() {
                        if msg.from == client.id {
                            continue;
                        }
                        let stream = &mut client.stream;
                        stream.write_all(msg.text.as_bytes()).unwrap();
                    }
                }
                Action::TerminateConnection(id) => {
                    let mut guard = also_clients.lock().unwrap();
                    guard.remove(&id);
                }
            }
        }
    });

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let client = Client {
                    id: Uuid::new_v4(),
                    stream,
                };

                let client_cloned = Client {
                    id: client.id,
                    stream: client.stream.try_clone().unwrap(),
                };
                let tx = tx.clone();
                let mut clients_guard = clients.lock().unwrap();
                clients_guard.insert(client.id, client_cloned);
                // clients_guard.insert(client.id, client_clone);

                thread::spawn(|| handle_connection(client, tx));
            }
            Err(e) => println!("Connection failed: {}", e),
        }
    }
}

fn handle_connection(mut client: Client, public_room: Sender<Action>) {
    println!(
        "connection established: {}",
        client.stream.peer_addr().unwrap()
    );
    let mut buff: [u8; 1024] = [0; 1024];

    // client.stream.set_nonblocking(true).unwrap();

    loop {
        match client.stream.read(&mut buff) {
            Ok(0) => {
                println!("Client {:?} disconnect", client.stream.peer_addr());
                public_room
                    .send(Action::TerminateConnection(client.id))
                    .unwrap();
                break;
            }
            Ok(n) => {
                let msg = String::from_utf8_lossy(&buff[0..n]);
                let msg = Message {
                    text: msg.to_string(),
                    from: client.id,
                };
                public_room.send(Action::SendMessage(msg)).unwrap();
            }
            // Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
            //     thread::sleep(Duration::from_millis(100));
            // }
            Err(err) => {
                println!("An error occured: {}. terminating the connection", err);
                public_room
                    .send(Action::TerminateConnection(client.id))
                    .unwrap();
                return;
            }
        }
    }
}
