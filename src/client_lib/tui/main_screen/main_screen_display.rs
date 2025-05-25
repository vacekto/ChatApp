use crate::{
    client_lib::{
        tui::app::app::App,
        util::{
            config::{
                MESSAGES_SCROLL_RESERVE, THEME_GRAY_GREEN_DARK, THEME_GREEN, THEME_YELLOW_1,
                THEME_YELLOW_2,
            },
            functions::pad_line_to_width,
            types::{ChannelKind, Contact, Focus},
        },
    },
    shared_lib::types::TuiMsg,
};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{
        Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget,
        Wrap,
    },
};

impl App {
    pub fn render_main_screen(&mut self, area: Rect, buf: &mut Buffer) {
        self.main_text_area.set_cursor_line_style(Style::default());

        let layout_main = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints(vec![Constraint::Fill(1), Constraint::Length(3)])
            .split(area.inner(Margin::new(1, 1)));

        let area_content = layout_main[0];
        let area_bottom_bar = layout_main[1];

        let layout_content = Layout::default()
            .direction(Direction::Horizontal)
            .margin(0)
            .constraints(vec![Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(area_content);

        let area_contacts = layout_content[0];
        let area_messages_and_intput = layout_content[1];

        let span_option_tab = Span::from("switch focus: Tab ");
        Line::from(vec![span_option_tab])
            .alignment(Alignment::Left)
            .render(area_bottom_bar.inner(Margin::new(1, 1)), buf);

        Block::default()
            .style(Style::default().bg(Color::Rgb(
                THEME_GRAY_GREEN_DARK.0,
                THEME_GRAY_GREEN_DARK.1,
                THEME_GRAY_GREEN_DARK.2,
            )))
            .render(area_bottom_bar, buf);

        let style_messages_title = match self.focus {
            Focus::Contacts => Style::default()
                .fg(Color::Rgb(
                    THEME_YELLOW_2.0,
                    THEME_YELLOW_2.1,
                    THEME_YELLOW_2.2,
                ))
                .bg(Color::Rgb(
                    THEME_GRAY_GREEN_DARK.0,
                    THEME_GRAY_GREEN_DARK.1,
                    THEME_GRAY_GREEN_DARK.2,
                )),
            Focus::Messages => Style::default()
                .bg(Color::Rgb(THEME_GREEN.0, THEME_GREEN.1, THEME_GREEN.2))
                .fg(Color::Rgb(
                    THEME_GRAY_GREEN_DARK.0,
                    THEME_GRAY_GREEN_DARK.1,
                    THEME_GRAY_GREEN_DARK.2,
                )),
        }
        .bold();

        let messages_title_text = match (self.active_channel.id, &self.active_channel.kind) {
            (None, _) => "".to_string(),
            (Some(_), ChannelKind::Direct) => {
                let channel = self
                    .direct_channels
                    .iter()
                    .find(|c| c.user.id == self.active_channel.id.unwrap());
                match channel {
                    None => "".to_string(),
                    Some(c) => format!(" {} ", c.user.username.clone()),
                }
            }
            (Some(_), ChannelKind::Room) => {
                let channel = self
                    .room_channels
                    .iter()
                    .find(|c| c.id == self.active_channel.id.unwrap());
                match channel {
                    None => "".to_string(),
                    Some(c) => format!(" {} ", c.name.clone()),
                }
            }
        };

        let title_messages = Span::styled(format!("{}", messages_title_text), style_messages_title);

        let style_contacts_title = match self.focus {
            Focus::Messages => Style::default()
                .fg(Color::Rgb(
                    THEME_YELLOW_2.0,
                    THEME_YELLOW_2.1,
                    THEME_YELLOW_2.2,
                ))
                .bg(Color::Rgb(
                    THEME_GRAY_GREEN_DARK.0,
                    THEME_GRAY_GREEN_DARK.1,
                    THEME_GRAY_GREEN_DARK.2,
                )),
            Focus::Contacts => Style::default()
                .bg(Color::Rgb(THEME_GREEN.0, THEME_GREEN.1, THEME_GREEN.2))
                .fg(Color::Rgb(
                    THEME_GRAY_GREEN_DARK.0,
                    THEME_GRAY_GREEN_DARK.1,
                    THEME_GRAY_GREEN_DARK.2,
                )),
        }
        .bold();

        let title_contacts_text = match self.active_channel.kind {
            ChannelKind::Direct => " Users ",
            ChannelKind::Room => " Rooms ",
        };

        let title_contacts = Span::styled(title_contacts_text, style_contacts_title);

        let title_input = Span::styled(
            " Input ",
            Style::default()
                .fg(Color::Rgb(
                    THEME_GRAY_GREEN_DARK.0,
                    THEME_GRAY_GREEN_DARK.1,
                    THEME_GRAY_GREEN_DARK.2,
                ))
                .bg(Color::Rgb(THEME_GREEN.0, THEME_GREEN.1, THEME_GREEN.2))
                .bold(), // Set background of the title
        );

        let layout_messages_and_intput = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Fill(1), Constraint::Length(5)])
            .split(area_messages_and_intput);

        let area_messages = layout_messages_and_intput[0].inner(Margin::new(0, 0));
        let area_input = layout_messages_and_intput[1].inner(Margin::new(0, 0));
        self.main_text_area
            .render(area_input.inner(Margin::new(2, 2)), buf);

        Block::bordered()
            .title(title_input)
            .border_set(border::PLAIN)
            .border_style(Style::default().fg(Color::Rgb(
                THEME_GREEN.0,
                THEME_GREEN.1,
                THEME_GREEN.2,
            )))
            .render(area_input, buf);

        let style_contacts_border = match self.focus {
            Focus::Contacts => {
                Style::default().fg(Color::Rgb(THEME_GREEN.0, THEME_GREEN.1, THEME_GREEN.2))
            }
            Focus::Messages => Style::default().fg(Color::Rgb(
                THEME_GRAY_GREEN_DARK.0,
                THEME_GRAY_GREEN_DARK.1,
                THEME_GRAY_GREEN_DARK.2,
            )),
        };

        let contacts_block = Block::bordered()
            .title(title_contacts)
            .border_set(border::PLAIN)
            .border_style(style_contacts_border);

        let mut contacts: Vec<Line> = vec![];

        match self.active_channel.kind {
            ChannelKind::Direct => {
                for c in &self.direct_channels {
                    let contact = Contact::Direct(c);
                    let mut contact_option = pad_line_to_width(Line::from(&contact), area.width);
                    match self.active_channel.id {
                        Some(id) if id == c.user.id => {
                            contact_option.style = Style::default()
                                .fg(Color::Rgb(
                                    THEME_YELLOW_1.0,
                                    THEME_YELLOW_1.1,
                                    THEME_YELLOW_1.2,
                                ))
                                .bg(Color::Rgb(
                                    THEME_YELLOW_2.0,
                                    THEME_YELLOW_2.1,
                                    THEME_YELLOW_2.2,
                                ))
                                .bold();
                        }
                        _ => {}
                    }
                    contacts.push(Line::from(contact_option))
                }
            }
            ChannelKind::Room => {
                for c in &self.room_channels {
                    let room = Contact::Room(c);
                    let mut contact_item = pad_line_to_width(Line::from(&room), area.width);
                    match self.active_channel.id {
                        Some(id) if id == c.id => {
                            contact_item.style = Style::default()
                                .fg(Color::Rgb(
                                    THEME_YELLOW_1.0,
                                    THEME_YELLOW_1.1,
                                    THEME_YELLOW_1.2,
                                ))
                                .bg(Color::Rgb(
                                    THEME_YELLOW_2.0,
                                    THEME_YELLOW_2.1,
                                    THEME_YELLOW_2.2,
                                ))
                                .bold();
                        }
                        _ => {}
                    }
                    contacts.push(Line::from(contact_item))
                }
            }
        }

        Paragraph::new(contacts)
            .block(contacts_block)
            .render(area_contacts, buf);

        let style_messages_border = match self.focus {
            Focus::Contacts => Style::default().fg(Color::Rgb(
                THEME_GRAY_GREEN_DARK.0,
                THEME_GRAY_GREEN_DARK.1,
                THEME_GRAY_GREEN_DARK.2,
            )),
            Focus::Messages => {
                Style::default().fg(Color::Rgb(THEME_GREEN.0, THEME_GREEN.1, THEME_GREEN.2))
            }
        };

        let block_messages = Block::bordered()
            .title(title_messages)
            .border_set(border::PLAIN)
            .border_style(style_messages_border);

        let mut messages: Vec<Line> = vec![];

        if let Some(id) = &self.active_channel.id {
            match &self.active_channel.kind {
                ChannelKind::Direct => {
                    match self.direct_channels.iter().find(|c| &c.user.id == id) {
                        None => {}
                        Some(c) => {
                            for m in c.messages.iter() {
                                match m {
                                    TuiMsg::Img(img) => {
                                        for line in img.cache.lines() {
                                            messages.push(line.into());
                                        }
                                    }
                                    TuiMsg::JoinNotification(n) => {
                                        messages.push(n.into());
                                    }
                                    TuiMsg::TextMsg(msg) => {
                                        messages.push(msg.into());
                                    }
                                }
                            }
                        }
                    }
                }
                ChannelKind::Room => match self.room_channels.iter().find(|c| &c.id == id) {
                    None => {}
                    Some(c) => {
                        for m in c.messages.iter() {
                            match m {
                                TuiMsg::Img(img) => {
                                    for line in img.cache.lines() {
                                        messages.push(line.into());
                                    }
                                }
                                TuiMsg::JoinNotification(n) => {
                                    messages.push(n.into());
                                }
                                TuiMsg::TextMsg(msg) => {
                                    messages.push(msg.into());
                                }
                            }
                        }
                    }
                },
            };
        };

        let mut scrollbar_state =
            ScrollbarState::new(MESSAGES_SCROLL_RESERVE).position(self.main_scroll_offset.into());

        let scrollbar: Scrollbar<'_> = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .orientation(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(Color::Rgb(
                THEME_GREEN.0,
                THEME_GREEN.1,
                THEME_GREEN.2,
            )))
            .track_style(
                Style::default()
                    .fg(Color::Rgb(
                        THEME_GRAY_GREEN_DARK.0,
                        THEME_GRAY_GREEN_DARK.1,
                        THEME_GRAY_GREEN_DARK.2,
                    ))
                    .bg(Color::Rgb(
                        THEME_GRAY_GREEN_DARK.0,
                        THEME_GRAY_GREEN_DARK.1,
                        THEME_GRAY_GREEN_DARK.2,
                    )),
            )
            .thumb_symbol("█");

        let scrollbar_area = area_messages.inner(Margin::new(2, 0));
        scrollbar.render(scrollbar_area, buf, &mut scrollbar_state);

        Paragraph::new(messages)
            .block(block_messages)
            .alignment(ratatui::layout::Alignment::Left)
            .wrap(Wrap { trim: false })
            .scroll((self.main_scroll_offset as u16, 0))
            .render(area_messages, buf);
    }
}
