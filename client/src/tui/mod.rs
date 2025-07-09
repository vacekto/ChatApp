pub mod accessories;
pub mod app;
pub mod entry_screen;
pub mod main_screen;
use anyhow::Result;
use app::app::App;

use crate::util::types::TuiUpdate;
use shared::types::{Chunk, ClientServerConnectMsg, ClientServerMsg};

pub async fn app(
    rx_ws_tui: tokio::sync::mpsc::Receiver<TuiUpdate>,
    tx_tui_ws_file: tokio::sync::mpsc::Sender<Chunk>,
    tx_tui_ws_msg: tokio::sync::mpsc::Sender<ClientServerMsg>,
    tx_tui_ws_auth: tokio::sync::mpsc::Sender<ClientServerConnectMsg>,
) -> Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new(rx_ws_tui, tx_tui_ws_file, tx_tui_ws_msg, tx_tui_ws_auth);

    app.run(&mut terminal).await?;
    ratatui::restore();

    Ok(())
}
