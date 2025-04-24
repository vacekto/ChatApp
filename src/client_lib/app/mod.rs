pub mod app;
pub mod app_functions;
pub mod widgets;

use std::sync::mpsc::{self, Receiver};

use anyhow::Result;
use app::App;

use crate::shared_lib::types::{ClientServerMsg, ServerClientMsg};

pub fn ratatui(
    rx_read_tui: Receiver<ServerClientMsg>,
    tx_tui_write: mpsc::Sender<ClientServerMsg>,
    username: String,
) -> Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new(tx_tui_write, username);
    app.run(&mut terminal, rx_read_tui)?;
    ratatui::restore();
    Ok(())
}
