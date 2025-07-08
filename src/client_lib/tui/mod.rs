pub mod accessories;
pub mod app;
pub mod entry_screen;
pub mod main_screen;
use anyhow::Result;
use app::app::App;

use crate::{
    client_lib::util::types::TuiUpdate,
    shared_lib::types::{Chunk, ClientServerConnectMsg, ClientServerMsg},
};

pub async fn tui(
    rx_tcp_tui: tokio::sync::mpsc::Receiver<TuiUpdate>,
    tx_tui_tcp_file: tokio::sync::mpsc::Sender<Chunk>,
    tx_tui_tcp_msg: tokio::sync::mpsc::Sender<ClientServerMsg>,
    tx_tui_tcp_auth: tokio::sync::mpsc::Sender<ClientServerConnectMsg>,
) -> Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new(rx_tcp_tui, tx_tui_tcp_file, tx_tui_tcp_msg, tx_tui_tcp_auth);

    app.run(&mut terminal).await?;
    ratatui::restore();

    Ok(())
}
