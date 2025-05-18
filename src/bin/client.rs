use anyhow::Result;
use chat_app::{
    client_lib::{
        data_stream::handle_file_streaming,
        global_states::{
            app_state::init_global_state,
            console_logger::initialize_console_logger,
            thread_logger::{get_thread_logger, get_thread_runner},
        },
        read_server::listen_for_server,
        tui::tui,
        write_server::write_to_server,
    },
    shared_lib::config::SERVER_ADDR,
};
use std::{net::TcpStream, thread, time::Duration};

fn main() -> Result<()> {
    initialize_console_logger();
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

    init_global_state(tcp);

    let th_runner = get_thread_runner();
    let th_logger = get_thread_logger();

    th_runner.spawn("read server", true, || listen_for_server());
    th_runner.spawn("write server", true, || write_to_server());
    th_runner.spawn("file stream", true, || handle_file_streaming());
    th_runner.spawn("ratatui", true, || tui());

    // tui()?;

    th_logger.log_results();
    Ok(())
}
