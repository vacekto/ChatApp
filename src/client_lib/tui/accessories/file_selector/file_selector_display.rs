use super::file_selector::FileSelector;
use crate::client_lib::util::{
    config::{THEME_GRAY_GREEN_DARK, THEME_YELLOW_DARK, THEME_YELLOW_LIGHT},
    functions::pad_line_to_width,
    types::FileAction,
};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

impl Widget for &mut FileSelector {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = 70;
        let height = 19;

        let x = (area.width.saturating_sub(width)) / 2;
        let y = (area.height.saturating_sub(height)) / 2;

        let rect_outer = Rect::new(x, y, width, height);
        Clear.render(rect_outer, buf);

        let style_title = Style::new()
            .fg(Color::Rgb(
                THEME_GRAY_GREEN_DARK.0,
                THEME_GRAY_GREEN_DARK.1,
                THEME_GRAY_GREEN_DARK.2,
            ))
            .bg(Color::Rgb(
                THEME_YELLOW_DARK.0,
                THEME_YELLOW_DARK.1,
                THEME_YELLOW_DARK.2,
            ))
            .bold();

        let style_outer = Style::default().bg(Color::Rgb(
            THEME_GRAY_GREEN_DARK.0,
            THEME_GRAY_GREEN_DARK.1,
            THEME_GRAY_GREEN_DARK.2,
        ));

        let style_border_content = Style::default().fg(Color::Rgb(
            THEME_YELLOW_LIGHT.0,
            THEME_YELLOW_LIGHT.1,
            THEME_YELLOW_LIGHT.2,
        ));

        let layout_name = Layout::default()
            .direction(Direction::Vertical)
            .vertical_margin(1)
            // .horizontal_margin(10)
            .constraints(vec![Constraint::Fill(1), Constraint::Length(2)])
            .split(rect_outer);

        let rect_content = layout_name[0];

        let title = match &self.active_action {
            FileAction::ASCII => Span::styled(" Send ASCI image ", style_title),
            FileAction::File => Span::styled(" Send file ", style_title),
        };

        Block::default()
            .title(title)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_set(border::EMPTY)
            .style(style_outer)
            .render(rect_outer, buf);

        let files: Vec<Line> = self
            .entries
            .iter()
            .map(|e| pad_line_to_width(e.into(), area.width))
            .collect();

        let block_selector = Block::default()
            .borders(Borders::ALL)
            .border_set(border::PLAIN)
            .border_style(style_border_content);

        let last_line = Rect {
            x: rect_outer.x,
            y: rect_outer.y + rect_outer.height.saturating_sub(2),
            width: rect_outer.width,
            height: 1,
        };

        let span_switch = Span::from("switch: ").style(Style::new().fg(Color::DarkGray));
        let span_tab = Span::from("< Tab >    ").style(Style::new().fg(Color::White));
        let span_select = Span::from(" move: ").style(Style::new().fg(Color::DarkGray));
        let span_arrows = Span::from("< ←,→,↑,↓ >    ").style(Style::new().fg(Color::White));
        let span_submit = Span::from(" select: ").style(Style::new().fg(Color::DarkGray));
        let span_enter = Span::from("< Enter >").style(Style::new().fg(Color::White));

        let options_line = Line::from(vec![
            span_switch,
            span_tab,
            span_select,
            span_arrows,
            span_submit,
            span_enter,
        ])
        .centered();

        options_line.render(last_line, buf);

        let upper_view_bound = (self.scroll_offset + rect_content.height - 2) as usize;

        if self.selected_index >= upper_view_bound {
            self.scroll_offset += 1;
        }

        if self.selected_index < self.scroll_offset as usize {
            self.scroll_offset -= self.selected_index.abs_diff(self.scroll_offset as usize) as u16;
        }

        let scroll_offset: (u16, u16) = (self.scroll_offset, 0);
        Paragraph::new(files)
            .block(block_selector)
            .scroll(scroll_offset)
            .render(rect_content.inner(Margin::new(1, 0)), buf);
    }
}
