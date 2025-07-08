pub mod accessories;
pub mod app;
pub mod entry_screen;
pub mod main_screen;
use anyhow::Result;
use app::app::App;

pub fn app() -> Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new();

    app.run(&mut terminal)?;
    ratatui::restore();

    Ok(())
}
