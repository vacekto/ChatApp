use anyhow::Result;
use chat_app::{
    client_lib::{
        app::ratatui,
        functions::{read_server, write_server},
        global_states::thread_logger::{get_thread_logger, get_thread_runner},
    },
    server_lib::util::config::{SERVER_HOSTNAME, SERVER_PORT},
    shared_lib::{types::ClientServerMsg, util_functions::get_addr},
};
use std::{io::stdout, net::TcpStream, sync::mpsc, thread, time::Duration};

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

    let (tx_stdin_write, rx_stdin_write) = mpsc::channel::<ClientServerMsg>();
    let tx_init = tx_stdin_write.clone();

    let th_runner = get_thread_runner();

    th_runner.run("write server", || write_server(write_tcp, rx_stdin_write));
    th_runner.run("read server", || read_server(read_tcp));
    th_runner.run("ratatui", || ratatui());
    // th.run_in_thread("stdin", || read_stdin(tx_stdin_write));

    tx_init.send(ClientServerMsg::InitClient)?;

    let th_logger = get_thread_logger();
    th_logger.log_results(stdout(), false);

    Ok(())
}
