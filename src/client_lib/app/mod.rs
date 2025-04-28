pub mod app;
pub mod app_functions;
pub mod widgets;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use std::sync::mpsc::{self, Receiver};

use anyhow::Result;
use app::App;

use crate::shared_lib::types::{ClientServerMsg, InitClientData, ServerClientMsg};

pub fn ratatui(
    rx_read_tui: Receiver<ServerClientMsg>,
    tx_tui_write: mpsc::Sender<ClientServerMsg>,
    init_data: InitClientData,
) -> Result<()> {
    let mut terminal = ratatui::init();

    execute!(std::io::stdout(), EnableMouseCapture)?;
    let mut app = App::new(tx_tui_write, init_data);
    app.run(&mut terminal, rx_read_tui)?;
    ratatui::restore();
    execute!(std::io::stdout(), DisableMouseCapture)?;

    Ok(())
}
