use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, Borders, Widget},
};

use crate::client_lib::tui::app::app::App;

impl App {
    pub fn render_login_screen(&self, area: Rect, buf: &mut Buffer) {
        let width_o = 60;
        let height_o = 15;

        let x_o = (area.width.saturating_sub(width_o)) / 2;
        let y_o = (area.height.saturating_sub(height_o)) / 2;

        let outer_rect = Rect::new(x_o, y_o, width_o, height_o);

        let width_i = 50;
        let height_i = 3;

        let x_i = (area.width.saturating_sub(width_i)) / 2;
        let y_i = (area.height.saturating_sub(height_i)) / 2;

        let inner_rect = Rect::new(x_i, y_i, width_i, height_i);

        Block::default()
            .title("outer")
            .borders(Borders::ALL)
            .render(outer_rect, buf);

        Block::default()
            .title("inner")
            .borders(Borders::ALL)
            .render(inner_rect, buf);

        // textarea.set_block(block);

        self.login_text_area.render(inner_rect, buf);
    }
}
