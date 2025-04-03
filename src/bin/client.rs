use std::{
    env,
    error::Error,
    io::{stdin, Read, Write},
    net::TcpStream,
    sync::mpsc::{self, Sender},
    thread,
    time::Duration,
};

const DEFUALT_HOSTNAME: &str = "localhost";
const DEFUALT_PORT: &str = "11111";

fn main() -> Result<(), Box<dyn Error>> {
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

    let read_stream = loop {
        println!("attempting to establish connection..");
        match TcpStream::connect(&addr) {
            Ok(s) => break s,
            Err(e) => {
                println!("connection error: {}", e);
                thread::sleep(Duration::from_secs(2));
            }
        }
    };

    println!("connection established with: {}", addr);

    let (read_tx, rx) = mpsc::channel();

    let write_tx = read_tx.clone();

    let write_stream = read_stream.try_clone().unwrap();

    let read_handle = thread::spawn(|| handle_read(read_stream, read_tx));

    let write_handle = thread::spawn(move || handle_write(write_stream, write_tx));

    rx.recv().unwrap();

    write_handle.join().unwrap();
    read_handle.join().unwrap();

    println!("finished");

    Ok(())
}

fn handle_write(mut stream: TcpStream, tx: Sender<&str>) {
    let mut write_buff = String::new();
    let reader = stdin();

    while let Ok(_) = reader.read_line(&mut write_buff) {
        if write_buff == ".quit" {
            tx.send("quit").unwrap();
            break;
        }

        match stream.write(write_buff.as_bytes()) {
            Ok(0) => {
                println!("server dropped");
                tx.send("quit").unwrap();
            }
            Ok(_) => {}
            Err(err) => {
                println!("server error occured: {:?}", err);
                tx.send("quit").unwrap();
                break;
            }
        };
        stream.flush().unwrap();
        write_buff.clear();
    }
}

fn handle_read(mut stream: TcpStream, tx: Sender<&str>) {
    let mut buff: [u8; 1024] = [0; 1024];

    loop {
        match stream.read(&mut buff) {
            Ok(0) => {
                println!("server dropped");
                tx.send("quit").unwrap();
                return;
            }
            Ok(n) => {
                let msg = String::from_utf8_lossy(&buff[0..n]);
                println!("{}", msg);
            }

            Err(err) => {
                println!("An error occured: {}. terminating the connection", err);
                tx.send("quit").unwrap();
                return;
            }
        }
    }
}
