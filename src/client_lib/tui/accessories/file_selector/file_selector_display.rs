use crate::client_lib::util::{
    config::{THEME_BG_DARK, THEME_BORDER},
    functions::pad_line_to_width,
    types::FileSelector,
};
use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

impl Widget for &mut FileSelector {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let width = 70;
        let height = 15;

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

        let title = Span::styled(
            " File selector ",
            Style::default()
                .fg(Color::Rgb(
                    THEME_BG_DARK.0,
                    THEME_BG_DARK.1,
                    THEME_BG_DARK.2,
                ))
                .bg(Color::Rgb(THEME_BORDER.0, THEME_BORDER.1, THEME_BORDER.2))
                .bold(),
        );

        let selector_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_set(border::PLAIN)
            .border_style(Style::default().fg(Color::Rgb(
                THEME_BORDER.0,
                THEME_BORDER.1,
                THEME_BORDER.2,
            )));

        let files: Vec<Line> = self
            .entries
            .iter()
            .map(|e| pad_line_to_width(e.into(), area.width))
            .collect();

        let scroll_offset: (u16, u16) = (self.scroll_offset, 0);

        Paragraph::new(files)
            .style(Style::default().bg(Color::Rgb(
                THEME_BG_DARK.0,
                THEME_BG_DARK.1,
                THEME_BG_DARK.2,
            )))
            .block(selector_block)
            .scroll(scroll_offset)
            .render(outer_rect, buf);
    }
}
