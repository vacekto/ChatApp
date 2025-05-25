use ratatui::widgets::Widget;

use crate::client_lib::util::types::ActiveScreen;

use super::app::App;

impl Widget for &mut App {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        match self.active_screen {
            ActiveScreen::Entry => self.render_entry_screen(area, buf),
            ActiveScreen::Main => self.render_main_screen(area, buf),
        }
    }
}
