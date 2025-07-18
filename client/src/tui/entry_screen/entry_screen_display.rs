use crate::{
    tui::app::app::App,
    util::{
        config::{THEME_GRAY_GREEN_DARK, THEME_GREEN, THEME_YELLOW_DARK, THEME_YELLOW_LIGHT},
        types::{ActiveEntryInput, ActiveEntryScreen, Notification},
    },
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Widget, Wrap},
};

impl App {
    pub fn render_entry_screen(&mut self, area: Rect, buf: &mut Buffer) {
        self.password_ta_login.set_mask_char('•');
        self.password_ta_register.set_mask_char('•');
        self.repeat_password_ta.set_mask_char('•');

        let width = 70;
        let height = 20;

        let x = (area.width.saturating_sub(width)) / 2;
        let y = (area.height.saturating_sub(height)) / 2;

        let rect_main = Rect::new(x, y, width, height);
        let style_main_border = Style::new().fg(Color::Rgb(
            THEME_YELLOW_LIGHT.0,
            THEME_YELLOW_LIGHT.1,
            THEME_YELLOW_LIGHT.2,
        ));

        Block::bordered()
            .border_set(border::DOUBLE)
            .border_style(style_main_border)
            .render(rect_main, buf);

        let layout_headline = Layout::default()
            .direction(Direction::Vertical)
            .vertical_margin(2)
            .horizontal_margin(4)
            .constraints(vec![Constraint::Length(3), Constraint::Fill(0)])
            .split(rect_main);

        let layout_username = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(3), Constraint::Fill(0)])
            .split(layout_headline[1]);

        let layout_password = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(3), Constraint::Fill(0)])
            .split(layout_username[1]);

        let layout_password_repeat = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(4), Constraint::Fill(0)])
            .split(layout_password[1]);

        let layout_notification = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(5), Constraint::Fill(0)])
            .split(layout_password_repeat[1]);

        let wrapper_headline = layout_headline[0];
        let wrapper_username = layout_username[0];
        let wrapper_password = layout_password[0];
        let wrapper_password_repeat = layout_password_repeat[0];
        let wrapper_notification = layout_notification[0];

        // Block::bordered()
        //     .border_set(border::DOUBLE)
        //     .border_style(style_main_border)
        //     .render(wrapper_headline, buf);

        let rect_headline = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(20), Constraint::Fill(0)])
            .split(wrapper_headline)[0];

        let rect_username = Layout::default()
            .direction(Direction::Vertical)
            .horizontal_margin(10)
            .constraints(vec![Constraint::Length(3), Constraint::Fill(0)])
            .split(wrapper_username)[0];

        let rect_password = Layout::default()
            .direction(Direction::Vertical)
            .horizontal_margin(10)
            .constraints(vec![Constraint::Length(3), Constraint::Fill(0)])
            .split(wrapper_password)[0];

        let rect_password_repeat = Layout::default()
            .direction(Direction::Vertical)
            .horizontal_margin(10)
            .constraints(vec![Constraint::Length(3), Constraint::Fill(0)])
            .split(wrapper_password_repeat)[0];

        let rect_notification = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(3), Constraint::Fill(0)])
            .split(wrapper_notification)[0];

        let headline_text = if self.active_entry_screen == ActiveEntryScreen::ASLogin {
            "Login"
        } else {
            "Register"
        };

        Text::from(vec![
            Line::from(
                Span::from(headline_text)
                    .style(Style::default().fg(Color::Rgb(
                        THEME_YELLOW_DARK.0,
                        THEME_YELLOW_DARK.1,
                        THEME_YELLOW_DARK.2,
                    )))
                    .bold(),
            )
            .centered(),
        ])
        .render(rect_headline, buf);

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

        self.username_ta_login
            .set_cursor_line_style(Style::default());
        self.password_ta_login
            .set_cursor_line_style(Style::default());
        self.username_ta_register
            .set_cursor_line_style(Style::default());
        self.password_ta_register
            .set_cursor_line_style(Style::default());
        self.repeat_password_ta
            .set_cursor_line_style(Style::default());

        match self.active_entry_input {
            ActiveEntryInput::Username => {
                self.username_ta_login
                    .set_cursor_style(style_active_cursor.clone());
                self.username_ta_register
                    .set_cursor_style(style_active_cursor.clone());
                self.password_ta_login
                    .set_cursor_style(style_empty_cursor.clone());
                self.password_ta_register
                    .set_cursor_style(style_empty_cursor.clone());
                self.repeat_password_ta
                    .set_cursor_style(style_empty_cursor.clone());
            }
            ActiveEntryInput::Password => {
                self.username_ta_login
                    .set_cursor_style(style_empty_cursor.clone());
                self.username_ta_register
                    .set_cursor_style(style_empty_cursor.clone());
                self.password_ta_login
                    .set_cursor_style(style_active_cursor.clone());
                self.password_ta_register
                    .set_cursor_style(style_active_cursor.clone());
                self.repeat_password_ta
                    .set_cursor_style(style_empty_cursor.clone());
            }
            ActiveEntryInput::RepeatPassword => {
                self.username_ta_register
                    .set_cursor_style(style_empty_cursor.clone());
                self.password_ta_register
                    .set_cursor_style(style_empty_cursor.clone());
                self.repeat_password_ta
                    .set_cursor_style(style_active_cursor.clone());
            }
        }

        match self.active_entry_screen {
            ActiveEntryScreen::ASLogin => {
                self.username_ta_login
                    .render(rect_username.inner(Margin::new(1, 1)), buf);
                self.password_ta_login
                    .render(rect_password.inner(Margin::new(1, 1)), buf);
            }
            ActiveEntryScreen::ASRegister => {
                self.username_ta_register
                    .render(rect_username.inner(Margin::new(1, 1)), buf);
                self.password_ta_register
                    .render(rect_password.inner(Margin::new(1, 1)), buf);
                self.repeat_password_ta
                    .render(rect_password_repeat.inner(Margin::new(1, 1)), buf);
            }
        }

        if let Some(notification) = &self.login_screen_notification {
            let (color, text) = match notification {
                Notification::Success(msg) => (Color::Green, msg),
                Notification::Failure(msg) => (Color::LightRed, msg),
            };

            Paragraph::new(Line::styled(text.clone(), Style::new().fg(color)))
                .centered()
                .wrap(Wrap { trim: true })
                .render(rect_notification, buf);
        }

        let style_input_title = Style::default()
            .fg(Color::Rgb(THEME_GREEN.0, THEME_GREEN.1, THEME_GREEN.2))
            .bold();
        let style_input_border = Style::default().fg(Color::Rgb(
            THEME_YELLOW_LIGHT.0,
            THEME_YELLOW_LIGHT.1,
            THEME_YELLOW_LIGHT.2,
        ));

        let title_username = Span::styled(" Username ", style_input_title.clone());
        Block::bordered()
            .border_set(border::PLAIN)
            .title(title_username)
            .border_style(style_input_border.clone())
            .render(rect_username, buf);

        let title_password = Span::styled(" Password ", style_input_title.clone());
        Block::bordered()
            .border_set(border::PLAIN)
            .title(title_password)
            .border_style(style_input_border.clone())
            .render(rect_password, buf);

        if self.active_entry_screen == ActiveEntryScreen::ASRegister {
            let title_password_repeat =
                Span::styled(" Repeat password ", style_input_title.clone());
            Block::bordered()
                .border_set(border::PLAIN)
                .title(title_password_repeat)
                .border_style(style_input_border.clone())
                .render(rect_password_repeat, buf);
        }

        let last_line = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(2),
            width: area.width,
            height: 1,
        };
        let span_switch = Span::from("switch: ").style(Style::new().fg(Color::DarkGray));
        let span_switch_s = Span::from("< Tab >    ").style(Style::new().fg(Color::White));
        let span_select = Span::from(" select: ").style(Style::new().fg(Color::DarkGray));
        let span_select_s = Span::from("< ↑,↓ >    ").style(Style::new().fg(Color::White));
        let span_submit = Span::from(" submit: ").style(Style::new().fg(Color::DarkGray));
        let span_submit_s = Span::from("< Enter >    ").style(Style::new().fg(Color::White));
        let span_logout = Span::from(" logout: ").style(Style::new().fg(Color::DarkGray));
        let span_logout_s = Span::from("< Esc >").style(Style::new().fg(Color::White));

        let options_line = Line::from(vec![
            span_switch,
            span_switch_s,
            span_select,
            span_select_s,
            span_submit,
            span_submit_s,
            span_logout,
            span_logout_s,
        ])
        .centered();

        options_line.render(last_line, buf);
    }
}
