use super::create_room::RoomCreator;
use crate::util::{
    config::{THEME_GRAY_GREEN_DARK, THEME_GREEN, THEME_YELLOW_DARK, THEME_YELLOW_LIGHT},
    types::{ActiveCreateRoomInput, RoomAction},
};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Widget, Wrap},
};

impl Widget for &mut RoomCreator {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.room_password_ta.set_mask_char('•');

        let width = 70;
        let height = 19;

        let x = (area.width.saturating_sub(width)) / 2;
        let y = (area.height.saturating_sub(height)) / 2;

        let rect_outer = Rect::new(x, y, width, height);
        Clear.render(rect_outer, buf);

        let style_outer_border =
            Style::default().fg(Color::Rgb(THEME_GREEN.0, THEME_GREEN.1, THEME_GREEN.2));

        let style_bg = Style::default().bg(Color::Rgb(
            THEME_GRAY_GREEN_DARK.0,
            THEME_GRAY_GREEN_DARK.1,
            THEME_GRAY_GREEN_DARK.2,
        ));

        let style_input_border = Style::default().fg(Color::Rgb(
            THEME_YELLOW_LIGHT.0,
            THEME_YELLOW_LIGHT.1,
            THEME_YELLOW_LIGHT.2,
        ));

        let style_outer_title = Style::default()
            .fg(Color::Rgb(
                THEME_GRAY_GREEN_DARK.0,
                THEME_GRAY_GREEN_DARK.1,
                THEME_GRAY_GREEN_DARK.2,
            ))
            .bold()
            .bg(Color::Rgb(
                THEME_YELLOW_DARK.0,
                THEME_YELLOW_DARK.1,
                THEME_YELLOW_DARK.2,
            ));

        let style_input_title = Style::default().fg(Color::Rgb(
            THEME_YELLOW_DARK.0,
            THEME_YELLOW_DARK.1,
            THEME_YELLOW_DARK.2,
        ));

        let title_text = match self.active_action {
            RoomAction::Create => " Create room ",
            RoomAction::Join => " Join room ",
        };

        let span_outer_title = Span::styled(title_text, style_outer_title).bold();
        let span_name_title = Span::styled(" Room name ", style_input_title.clone()).bold();
        let span_password_title = Span::styled(" Room password ", style_input_title.clone()).bold();

        Block::bordered()
            .style(style_bg)
            .border_set(border::EMPTY)
            .title(span_outer_title)
            .title_alignment(Alignment::Center)
            .border_style(style_outer_border.clone())
            .render(rect_outer, buf);

        let layout_name = Layout::default()
            .direction(Direction::Vertical)
            .vertical_margin(3)
            .horizontal_margin(10)
            .constraints(vec![Constraint::Length(4), Constraint::Fill(0)])
            .split(rect_outer);

        let layout_password = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(5), Constraint::Fill(0)])
            .split(layout_name[1]);

        let layout_notification = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(6), Constraint::Fill(0)])
            .split(layout_password[1]);

        let wrapper_name = layout_name[0];
        let wrapper_password = layout_password[0];
        let wrapper_notification = layout_notification[0];

        let rect_name = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(3), Constraint::Fill(0)])
            .split(wrapper_name)[0];

        let rect_password = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(3), Constraint::Fill(0)])
            .split(wrapper_password)[0];

        let rect_notification = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(3), Constraint::Fill(0)])
            .split(wrapper_notification)[0];

        Block::bordered()
            .border_set(border::PLAIN)
            .title(span_name_title)
            .border_style(style_input_border.clone())
            .render(rect_name, buf);

        Block::bordered()
            .border_set(border::PLAIN)
            .title(span_password_title)
            .border_style(style_input_border.clone())
            .render(rect_password, buf);

        if let Some(text) = &self.notification {
            Paragraph::new(Line::styled(text.clone(), Style::new().fg(Color::LightRed)))
                .centered()
                .wrap(Wrap { trim: true })
                .render(rect_notification, buf);
        };

        self.room_name_ta.set_cursor_line_style(Style::default());
        self.room_password_ta
            .set_cursor_line_style(Style::default());

        let style_empty_cursor = Style::default();
        let style_active_cursor = Style::new()
            .fg(Color::Rgb(
                THEME_GRAY_GREEN_DARK.0,
                THEME_GRAY_GREEN_DARK.1,
                THEME_GRAY_GREEN_DARK.2,
            ))
            .bg(Color::Rgb(
                THEME_YELLOW_DARK.0,
                THEME_YELLOW_DARK.1,
                THEME_YELLOW_DARK.2,
            ));

        // .add_modifier(Modifier::UNDERLINED);

        match self.active_input {
            ActiveCreateRoomInput::Name => {
                self.room_name_ta
                    .set_cursor_style(style_active_cursor.clone());
                self.room_password_ta
                    .set_cursor_style(style_empty_cursor.clone());
            }
            ActiveCreateRoomInput::Password => {
                self.room_name_ta
                    .set_cursor_style(style_empty_cursor.clone());
                self.room_password_ta
                    .set_cursor_style(style_active_cursor.clone());
            }
        }

        self.room_name_ta
            .render(rect_name.inner(Margin::new(1, 1)), buf);
        self.room_password_ta
            .render(rect_password.inner(Margin::new(1, 1)), buf);

        let last_line = Rect {
            x: rect_outer.x,
            y: rect_outer.y + rect_outer.height.saturating_sub(2),
            width: rect_outer.width,
            height: 1,
        };

        let span_switch = Span::from("switch: ").style(Style::new().fg(Color::DarkGray));
        let span_tab = Span::from("< Tab >    ").style(Style::new().fg(Color::White));
        let span_select = Span::from(" select: ").style(Style::new().fg(Color::DarkGray));
        let span_arrows = Span::from("< ↑,↓ >    ").style(Style::new().fg(Color::White));
        let span_submit = Span::from(" submit: ").style(Style::new().fg(Color::DarkGray));
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
    }
}
