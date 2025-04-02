use std::{
    env,
    io::Read,
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

const DEFUALT_HOSTNAME: &str = "localhost";
const DEFUALT_PORT: &str = "11111";

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

    let server_up_flag = Arc::new(AtomicBool::new(true));

    println!("listening on: {}", addr);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // let server_up_flag = server_up_flag.clone();
                thread::spawn(|| handle_connection(stream));
            }
            Err(e) => println!("Connection failed: {}", e),
        }
    }

    server_up_flag.store(false, Ordering::Relaxed);
}

fn handle_connection(
    mut stream: TcpStream,
    // server_up_flag: Arc<AtomicBool>,
) -> Result<(), std::io::Error> {
    println!("connection established: {}", stream.peer_addr()?);
    let mut buff = [0; 1024];

    // stream.set_nonblocking(true).unwrap();

    loop {
        match stream.read(&mut buff) {
            Ok(0) => {
                println!("Client {:?} disconnect", stream.peer_addr());
                break;
            }
            Ok(n) => {
                let received_str = String::from_utf8_lossy(&buff[0..n]);
                println!("New message:  {}", received_str);
            }
            // Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
            //     if !server_up_flag.load(Ordering::Relaxed) {
            //         break;
            //     }
            //     thread::sleep(Duration::from_millis(100));
            //     println!("cosikdosi");
            // }
            Err(err) => {
                println!("An error occured: {}. terminating the connection", err);
                return Err(err);
            }
        };
    }
    Ok(())
}
