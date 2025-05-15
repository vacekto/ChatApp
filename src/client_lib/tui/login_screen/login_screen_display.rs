use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Widget},
};

use crate::client_lib::{tui::app::app::App, util::config::THEME_GREEN};

impl App {
    pub fn render_login_screen(&mut self, area: Rect, buf: &mut Buffer) {
        self.login_text_area.set_cursor_line_style(Style::default());

        let width_o = 60;
        let height_o = 10;

        let x_o = (area.width.saturating_sub(width_o)) / 2;
        let y_o = (area.height.saturating_sub(height_o)) / 2;

        let outer_rect = Rect::new(x_o, y_o, width_o, height_o);

        let layout_login = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(outer_rect);

        let lower_rect = layout_login[1].inner(Margin::new(0, 1));
        let upper_rect = layout_login[0];
        let inner_upper_rect = upper_rect.inner(Margin::new(1, 1));

        let title_msg = Span::styled(
            " Username ",
            Style::default()
                .fg(Color::Rgb(THEME_GREEN.0, THEME_GREEN.1, THEME_GREEN.2))
                .bold(),
        );

        if let Some(msg) = &self.login_notification {
            Line::from(msg.clone())
                .centered()
                .style(Style::default().fg(Color::LightRed))
                .render(lower_rect, buf);
        }

        Block::bordered()
            .title(title_msg)
            .border_set(border::PLAIN)
            .border_style(Style::default().fg(Color::Rgb(
                THEME_GREEN.0,
                THEME_GREEN.1,
                THEME_GREEN.2,
            )))
            .render(upper_rect, buf);

        self.login_text_area.render(inner_upper_rect, buf);
    }
}
