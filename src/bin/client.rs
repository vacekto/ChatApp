use anyhow::Result;
use chat_app::{
    client_lib::{
        global_states::{
            app_state::init_global_state,
            thread_logger::{get_thread_logger, get_thread_runner},
        },
        read_server::tcp_read,
        tui::tui,
        util::types::MpscChannel,
        write_server::tcp_write,
    },
    shared_lib::{
        config::SERVER_ADDR,
        types::{ServerTuiMsg, TuiServerMsg},
    },
};
use std::{net::TcpStream, sync::mpsc, thread, time::Duration};

fn main() -> Result<()> {
    let tcp = loop {
        println!("attempting to establish connection../");
        match TcpStream::connect(SERVER_ADDR) {
            Ok(s) => {
                println!("connection established with: :{}", SERVER_ADDR);
                break s;
            }
            Err(err) => {
                println!("connection error: {}", err);
                thread::sleep(Duration::from_secs(1));
            }
        }
    };

    let (tx_tcp_tui, rx_tcp_tui) = mpsc::channel::<ServerTuiMsg>();
    let (tx_tui_tcp, rx_tui_tcp) = mpsc::channel::<TuiServerMsg>();

    let tcp_tui_channel = MpscChannel {
        tx: tx_tcp_tui,
        rx: Some(rx_tcp_tui),
    };

    let tui_tcp_channel = MpscChannel {
        tx: tx_tui_tcp,
        rx: Some(rx_tui_tcp),
    };

    init_global_state(tcp, tcp_tui_channel, tui_tcp_channel);

    let th_runner = get_thread_runner();
    let th_logger = get_thread_logger();

    th_runner.spawn("write server", true, || tcp_write());
    th_runner.spawn("read server", true, || tcp_read());
    th_runner.spawn("ratatui", true, || tui());

    th_logger.log_results();
    Ok(())
}
