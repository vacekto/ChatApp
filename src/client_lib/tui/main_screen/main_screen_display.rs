use crate::client_lib::{
    tui::app::app::App,
    util::{
        config::{THEME_BG_DARK, THEME_BG_LIGHT, THEME_BORDER, THEME_SELECT, THEME_SELECT_BG},
        functions::pad_line_to_width,
        types::{ChannelKind, Contact},
    },
};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget, Wrap},
};

impl App {
    pub fn render_main_screen(&mut self, area: Rect, buf: &mut Buffer) {
        self.main_text_area.set_cursor_line_style(Style::default());

        let layout_main = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints(vec![Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(area);

        let title_msg = Span::styled(
            " Messages ",
            Style::default()
                .fg(Color::Rgb(
                    THEME_BG_DARK.0,
                    THEME_BG_DARK.1,
                    THEME_BG_DARK.2,
                ))
                .bg(Color::Rgb(THEME_BORDER.0, THEME_BORDER.1, THEME_BORDER.2))
                .bold(), // Set background of the title
        );

        let title_contacts_text = match self.active_channel.kind {
            ChannelKind::Direct => " Users ",
            ChannelKind::Room => " Rooms ",
        };

        let title_contacts = Span::styled(
            title_contacts_text,
            Style::default()
                .fg(Color::Rgb(
                    THEME_BG_DARK.0,
                    THEME_BG_DARK.1,
                    THEME_BG_DARK.2,
                ))
                .bg(Color::Rgb(THEME_BORDER.0, THEME_BORDER.1, THEME_BORDER.2))
                .bold(), // Set background of the title
        );

        let title_input = Span::styled(
            " Input ",
            Style::default()
                .fg(Color::Rgb(
                    THEME_BG_DARK.0,
                    THEME_BG_DARK.1,
                    THEME_BG_DARK.2,
                ))
                .bg(Color::Rgb(THEME_BORDER.0, THEME_BORDER.1, THEME_BORDER.2))
                .bold(), // Set background of the title
        );

        let area_contacts = layout_main[0].inner(Margin::new(0, 0));
        let area_right = layout_main[1].inner(Margin::new(0, 0));
        let layout_right = Layout::default()
            .direction(Direction::Vertical)
            // .vertical_margin(1)
            .constraints(vec![Constraint::Percentage(80), Constraint::Percentage(20)])
            .split(area_right);
        let area_messages = layout_right[0].inner(Margin::new(0, 0));
        let area_input = layout_right[1].inner(Margin::new(0, 0));
        let area_input_inner = area_input.inner(Margin::new(2, 2));
        self.main_text_area.render(area_input_inner, buf);
        // Block::default()
        //     .style(Style::default().bg(Color::Rgb(43, 51, 57)))
        //     // .borders(Borders::ALL)
        //     .render(area, buf);

        Block::bordered()
            .title(title_input)
            .border_set(border::PLAIN)
            .border_style(Style::default().fg(Color::Rgb(
                THEME_BORDER.0,
                THEME_BORDER.1,
                THEME_BORDER.2,
            )))
            .render(area_input, buf);

        let contacts_block = Block::bordered()
            .title(title_contacts)
            .border_set(border::PLAIN)
            .border_style(Style::default().fg(Color::Rgb(
                THEME_BORDER.0,
                THEME_BORDER.1,
                THEME_BORDER.2,
            )));

        let mut contacts: Vec<Line> = vec![];

        match self.active_channel.kind {
            ChannelKind::Direct => {
                for c in &self.direct_channels {
                    let contact = Contact::Direct(c);
                    let mut contact_line = pad_line_to_width(Line::from(&contact), area.width);
                    match self.active_channel.id {
                        Some(id) if id == c.user.id => {
                            contact_line.style = Style::default()
                                .fg(Color::Rgb(THEME_SELECT.0, THEME_SELECT.1, THEME_SELECT.2))
                                .bg(Color::Rgb(
                                    THEME_SELECT_BG.0,
                                    THEME_SELECT_BG.1,
                                    THEME_SELECT_BG.2,
                                ))
                                .bold();
                        }
                        _ => {}
                    }
                    contacts.push(Line::from(contact_line))
                }
            }
            ChannelKind::Room => {
                for c in &self.room_channels {
                    let room = Contact::Room(c);
                    let mut contact_line = pad_line_to_width(Line::from(&room), area.width);
                    match self.active_channel.id {
                        Some(id) if id == c.id => {
                            contact_line.style = Style::default()
                                .fg(Color::Rgb(THEME_SELECT.0, THEME_SELECT.1, THEME_SELECT.2))
                                .bg(Color::Rgb(
                                    THEME_SELECT_BG.0,
                                    THEME_SELECT_BG.1,
                                    THEME_SELECT_BG.2,
                                ))
                                .bold();
                        }
                        _ => {}
                    }
                    contacts.push(Line::from(contact_line))
                }
            }
        }

        Paragraph::new(contacts)
            .block(contacts_block)
            .style(Style::default().bg(Color::Rgb(
                THEME_BG_LIGHT.0,
                THEME_BG_LIGHT.1,
                THEME_BG_LIGHT.2,
            )))
            .render(area_contacts, buf);

        let messages_block = Block::bordered()
            .title(title_msg)
            .border_set(border::PLAIN)
            .border_style(Style::default().fg(Color::Rgb(
                THEME_BORDER.0,
                THEME_BORDER.1,
                THEME_BORDER.2,
            )));

        let mut messages: Vec<Line> = vec![];
        if let Some(id) = &self.active_channel.id {
            messages = match &self.active_channel.kind {
                ChannelKind::Direct => {
                    match self.direct_channels.iter().find(|c| &c.user.id == id) {
                        None => vec![],
                        Some(c) => c.messages.iter().map(|m| m.into()).collect(),
                    }
                }
                ChannelKind::Room => match self.room_channels.iter().find(|c| &c.id == id) {
                    None => vec![],
                    Some(c) => c.messages.iter().map(|m| m.into()).collect(),
                },
            };
        };
        Paragraph::new(messages)
            .block(messages_block)
            .wrap(Wrap { trim: true })
            .render(area_messages, buf);
    }
}
