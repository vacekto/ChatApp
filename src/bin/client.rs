use anyhow::Result;
use chat_app::client_lib::{
    data_stream::handle_file_streaming,
    global_states::{
        app_state::init_global_state,
        // console_logger::initialize_console_logger,
        thread_logger::get_thread_runner,
    },
    read_server::listen_for_server,
    tui::tui,
    write_server::write_to_server,
};
use std::{net::TcpStream, thread, time::Duration};

fn main() -> Result<()> {
    // initialize_console_logger();

    let server_addr = match std::env::var("SERVER_PORT") {
        Ok(port) => format!("localhost:{port}"),
        Err(_) => String::from("localhost:11111"),
    };

    let tcp = loop {
        println!("attempting to establish connection../");
        match TcpStream::connect(&server_addr) {
            Ok(s) => {
                println!("connection established with: :{}", server_addr);
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

    th_runner.spawn("read server", true, || listen_for_server());
    th_runner.spawn("write server", true, || write_to_server());
    th_runner.spawn("file stream", true, || handle_file_streaming());
    tui()?;

    // th_runner.spawn("ratatui", true, || tui());
    // let th_logger = get_thread_logger();
    // th_logger.log_results();
    Ok(())
}
