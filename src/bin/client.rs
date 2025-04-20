use anyhow::Result;
use chat_app::{
    client_lib::{
        functions::{read_server, read_stdin, run_in_thread, write_server},
        util::{errors::ThreadError, types::ThreadPurpuse},
    },
    server_lib::util::config::{SERVER_HOSTNAME, SERVER_PORT},
    shared_lib::{types::ClientToServerMsg, util_functions::get_addr},
};
use std::{net::TcpStream, sync::mpsc, thread, time::Duration};

fn main() -> Result<()> {
    let addr = get_addr(SERVER_HOSTNAME, SERVER_PORT);

    let read_tcp = loop {
        println!("attempting to establish connection..");
        match TcpStream::connect(&addr) {
            Ok(s) => {
                println!("connection established with: :{}", addr);
                break s;
            }
            Err(e) => {
                println!("connection error: {}", e);
                thread::sleep(Duration::from_secs(1));
            }
        }
    };

    let write_tcp = read_tcp.try_clone()?;

    let (tx_stdin_write, rx) = mpsc::channel::<ClientToServerMsg>();
    let tx_init = tx_stdin_write.clone();

    let (tx_thread_result, rx_thread_result) = mpsc::channel::<Result<String, ThreadError>>();

    run_in_thread(ThreadPurpuse::WriteServer, tx_thread_result.clone(), || {
        write_server(write_tcp, rx)
    });

    run_in_thread(ThreadPurpuse::ReadServer, tx_thread_result.clone(), || {
        read_server(read_tcp)
    });

    run_in_thread(ThreadPurpuse::StdIn, tx_thread_result.clone(), || {
        read_stdin(tx_stdin_write)
    });

    let init_msg = ClientToServerMsg::InitClient;
    tx_init.send(init_msg)?;

    loop {
        match rx_thread_result.recv().unwrap() {
            Ok(res) => println!("{}", res),
            Err(err) => {
                eprintln!("{}\n", err);
                match err {
                    ThreadError::ReadServer(err) => eprintln!("{}", err.backtrace()),
                    ThreadError::StdIn(err) => eprintln!("{}", err.backtrace()),
                    ThreadError::WriteServer(err) => eprintln!("{}", err.backtrace()),
                }
            }
        };
    }
}
