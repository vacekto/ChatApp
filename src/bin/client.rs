use anyhow::Result;
use chat_app::{
    client_lib::{
        app::ratatui,
        global_states::thread_logger::{get_thread_logger, get_thread_runner},
        read_server::tcp_read,
        write_server::tcp_write,
    },
    shared_lib::{
        config::SERVER_ADDR,
        types::{ClientServerMsg, ServerClientMsg},
    },
};
use std::{env, io::stdout, net::TcpStream, sync::mpsc, thread, time::Duration};

fn main() -> Result<()> {
    let username = env::args()
        .nth(1)
        .expect("provide username as ClI argument");
    let username_clone = username.clone();

    let read_tcp = loop {
        println!("attempting to establish connection..");
        match TcpStream::connect(SERVER_ADDR) {
            Ok(s) => {
                println!("connection established with: :{}", SERVER_ADDR);
                break s;
            }
            Err(e) => {
                println!("connection error: {}", e);
                thread::sleep(Duration::from_secs(1));
            }
        }
    };

    let write_tcp = read_tcp.try_clone()?;

    let (tx_tui_write, rx_tui_write) = mpsc::channel::<ClientServerMsg>();
    let (tx_read_tui, rx_read_tui) = mpsc::channel::<ServerClientMsg>();

    let tx_init = tx_tui_write.clone();

    let th_runner = get_thread_runner();

    th_runner.run("write server", || tcp_write(write_tcp, rx_tui_write));
    th_runner.run("read server", || tcp_read(read_tcp, tx_read_tui));
    th_runner.run("ratatui", || {
        ratatui(rx_read_tui, tx_tui_write, username_clone)
    });
    // th.run_in_thread("stdin", || read_stdin(tx_stdin_write));

    tx_init.send(ClientServerMsg::InitClient(username))?;

    let th_logger = get_thread_logger();
    th_logger.log_results(stdout(), true);

    Ok(())
}
