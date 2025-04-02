use std::{
    env,
    error::Error,
    io::{stdin, Write},
    net::TcpStream,
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

    let mut stream = loop {
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

    let mut write_buff = String::new();
    let reader = stdin();

    while let Ok(_) = reader.read_line(&mut write_buff) {
        if write_buff == ".quit" {
            break;
        }

        match stream.write(write_buff.as_bytes()) {
            Ok(0) => {
                println!("server dropped");
            }
            Ok(n) => {
                println!("{}", n);
            }
            Err(err) => {
                println!("server error occured: {:?}", err);
                break;
            }
        };
        stream.flush().unwrap();
        write_buff.clear();
    }

    println!("finished");

    Ok(())
}
