use anyhow::Result;
use chat_app::{
    client_lib::{
        app::ratatui,
        global_states::{
            console_logger::close_console_logger,
            thread_logger::{get_thread_logger, get_thread_runner, init_thread_logger},
        },
        read_server::tcp_read,
        util::functions::handle_auth,
        write_server::tcp_write,
    },
    shared_lib::{
        config::SERVER_ADDR,
        types::{TuiServerMsg, ServerTuiMsg},
    },
};
use std::{env, net::TcpStream, sync::mpsc, thread, time::Duration};

fn main() -> Result<()> {
    init_thread_logger();

    let username = env::args()
        .nth(1)
        .expect("provide username as ClI argument");

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

    let (tx_tui_write, rx_tui_write) = mpsc::channel::<TuiServerMsg>();
    let (tx_read_tui, rx_read_tui) = mpsc::channel::<ServerTuiMsg>();

    let init_data = handle_auth(write_tcp.try_clone()?, username)?;

    let th_runner = get_thread_runner();

    th_runner.run("write server", || tcp_write(write_tcp, rx_tui_write));
    th_runner.run("read server", || tcp_read(read_tcp, tx_read_tui));
    th_runner.run("ratatui", || ratatui(rx_read_tui, tx_tui_write, init_data));

    let th_logger = get_thread_logger();
    th_logger.log_results(true);
    close_console_logger();
    Ok(())
}
