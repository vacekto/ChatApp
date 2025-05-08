pub mod accessories;
pub mod app;
pub mod login_screen;
pub mod main_screen;

use anyhow::Result;
use app::app::App;

pub fn tui() -> Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new();

    app.run(&mut terminal)?;
    ratatui::restore();

    Ok(())
}
