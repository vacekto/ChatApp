use crate::client_lib::util::types::FileSelector;
use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, Widget},
};

impl Widget for &mut FileSelector {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        // self.update_entries().unwrap();

        let width = 80;
        let height = 10;

        let lower_bound = (self.scroll_offset + height - 2) as usize;

        if self.selected_index >= lower_bound {
            self.scroll_offset += 1;
        }

        if self.selected_index < self.scroll_offset as usize {
            self.scroll_offset -= self.selected_index.abs_diff(self.scroll_offset as usize) as u16;
        }

        let x = (area.width.saturating_sub(width)) / 2;
        let y = (area.height.saturating_sub(height)) / 2;

        let outer_rect = Rect::new(x, y, width, height);

        let selector_block = Block::default()
            .title("file selector")
            .borders(Borders::ALL);

        let files: Vec<Line> = self.entries.iter().map(|e| e.into()).collect();

        let scroll_offset: (u16, u16) = (self.scroll_offset, 0);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        Paragraph::new(files)
            .block(selector_block)
            .scroll(scroll_offset)
            .render(outer_rect, buf);
    }
}
