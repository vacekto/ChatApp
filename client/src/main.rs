use anyhow::Result;
use client::{
    data_stream::handle_file_streaming,
    global_states::{app_state::init_global_state, thread_logger::get_thread_runner},
    read_server::listen_for_server,
    tui::app,
    write_server::write_to_server,
};
use dotenv::dotenv;
use std::{net::TcpStream, thread, time::Duration};

fn main() -> Result<()> {
    dotenv().ok();
    // initialize_console_logger();

    let server_addr = format!(
        "{}:{}",
        std::env::var("SERVER_HOST")?,
        std::env::var("SERVER_PORT")?
    );

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
    app()?;

    // th_runner.spawn("ratatui", true, || tui());
    // let th_logger = get_thread_logger();
    // th_logger.log_results();
    Ok(())
}
