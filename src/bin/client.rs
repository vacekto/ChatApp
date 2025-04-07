use chat_app::shared_lib::{get_addr, BUFF_LENGTH};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
    error::Error,
    fs::File,
    io::{stdin, Read, Write},
    net::TcpStream,
    path::Path,
    sync::mpsc::{self, Sender},
    thread,
    time::Duration,
};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
struct FileMetadata {
    filename: String,
    file_length: u64,
}

#[derive(Serialize, Deserialize, Debug)]
enum MsgMetadata {
    File(FileMetadata),
    Text,
}

use tokio::{sync::broadcast, task};

const DEFUALT_HOSTNAME: &str = "localhost";
const DEFUALT_PORT: &str = "11111";

fn main() -> Result<(), Box<dyn Error>> {
    let addr = get_addr(DEFUALT_HOSTNAME, DEFUALT_PORT);

    let in_tcp = loop {
        println!("attempting to establish connection..");
        match TcpStream::connect(&addr) {
            Ok(s) => {
                println!("connection established with: {}", addr);
                break s;
            }
            Err(e) => {
                println!("connection error: {}", e);
                thread::sleep(Duration::from_secs(2));
            }
        }
    };

    let (in_tx, rx) = mpsc::channel();
    let out_tx = in_tx.clone();
    let out_tcp = in_tcp.try_clone().unwrap();
    let in_th = thread::spawn(|| listen_for_server(in_tcp, in_tx));
    let out_th = thread::spawn(move || handle_stdin(out_tcp, out_tx));

    rx.recv().unwrap();

    out_th.join().unwrap();
    in_th.join().unwrap();

    println!("finished");

    Ok(())
}

fn handle_stdin(mut tcp: TcpStream, tx: Sender<()>) {
    let mut buff = String::new();
    let s_in = stdin();

    while let Ok(_) = s_in.read_line(&mut buff) {
        let mut itr = buff.split_whitespace();

        match (itr.next(), itr.next()) {
            (Some(cmd), None) if cmd == ".quit" => {
                tx.send(()).unwrap();
                return;
            }
            (Some(cmd), Some(_)) if cmd == ".file" && itr.count() != 0 => {
                println!("too many arguments, expected format <>.file> <command>")
            }
            (Some(cmd), Some(path)) if cmd == ".file" => {
                send_file(&mut tcp, path);
            }

            _ => {
                send_text_msg(&mut tcp, buff.clone());
            }
        }

        tcp.flush().unwrap();
        buff.clear();
    }
}

fn listen_for_server(mut tcp: TcpStream, tx: Sender<()>) {
    let mut buff: [u8; BUFF_LENGTH] = [0; BUFF_LENGTH];

    loop {
        match tcp.read(&mut buff) {
            Ok(0) => {
                println!("server dropped");
                tx.send(()).unwrap();
                return;
            }
            Ok(n) => {
                // let metadata = bincode::deserialize::<MsgMetadata>(&buff).unwrap();
                let msg = String::from_utf8_lossy(&buff[0..n]);
                println!("{}", msg);
                // println!("{:?}", metadata);
                // println!("{:?}", buff);
            }

            Err(err) => {
                println!("An error occured: {}. terminating the connection", err);
                tx.send(()).unwrap();
                return;
            }
        }
    }
}

fn send_file(tcp: &mut TcpStream, path: &str) {
    unimplemented!();

    // let path = Path::new(path);
    // let file = File::open(path).unwrap();
    // let filename = path.file_name().unwrap().to_str().unwrap().to_string();
    // let size = file.metadata().unwrap().len();

    // let metadata = MsgMetadata::File(FileMetadata {
    //     file_length: size,
    //     filename,
    // });

    // println!("{}", size);
    // println!("{:#?}", file.metadata().unwrap());

    // let bin = bincode::serialize(&metadata).unwrap();

    // tcp.write_all(&bin).unwrap();
}

fn send_text_msg(tcp: &mut TcpStream, msg: String) {
    // let mut buff: [u8; BUFF_LENGTH] = [0; BUFF_LENGTH];

    // let id = Uuid::new_v4();

    // let bin = bincode::serialize(&id).unwrap();
    // let ab = id.as_bytes();

    // let metadata = MsgMetadata::Text;

    // let bin = bincode::serialize(&metadata).unwrap();
    // tcp.write(&bin).unwrap();

    match tcp.write(msg.as_bytes()) {
        Ok(0) => {
            println!("server dropped");
            // tx.send(()).unwrap();
        }
        Ok(_) => {}
        Err(err) => {
            println!("server error occured: {:?}", err);
            // tx.send(()).unwrap();
            // break;
        }
    }
}
