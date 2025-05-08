use crate::client_lib::{
    tui::app::app::App,
    util::types::{ChannelKind, Contact},
};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget, Wrap},
};

impl App {
    pub fn render_main_screen(&self, area: Rect, buf: &mut Buffer) {
        let layout_main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(area);

        let title_msg = Line::from(" Messages ");
        let title_contacts = match self.active_channel.kind {
            ChannelKind::Direct => Line::from(" Users "),
            ChannelKind::Room => Line::from(" Rooms "),
        };

        let title_input = Line::from(" Input ");

        let area_contacts = layout_main[0].inner(Margin {
            horizontal: 2,
            vertical: 3,
        });

        let area_right = layout_main[1].inner(Margin {
            horizontal: 2,
            vertical: 3,
        });

        let layout_right = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(80), Constraint::Percentage(20)])
            .split(area_right);

        let area_messages = layout_right[0].inner(Margin {
            horizontal: 0,
            vertical: 0,
        });

        let area_input = layout_right[1].inner(Margin {
            horizontal: 0,
            vertical: 0,
        });

        let area_input_inner = area_input.inner(Margin {
            horizontal: 2,
            vertical: 2,
        });

        Block::bordered()
            .title(title_input)
            .border_set(border::PLAIN)
            .render(area_input, buf);

        let contacts_block = Block::bordered()
            .title(title_contacts.centered())
            .border_set(border::PLAIN);

        let mut contacts: Vec<Line> = vec![];

        match self.active_channel.kind {
            ChannelKind::Direct => {
                for c in &self.direct_channels {
                    let contact = Contact::Direct(c);
                    let mut contact_span = Span::from(&contact);
                    match self.active_channel.id {
                        Some(id) if id == c.user.id => {
                            contact_span.style =
                                Style::default().fg(Color::White).bg(Color::DarkGray);
                        }
                        _ => {}
                    }
                    contacts.push(Line::from(contact_span))
                }
            }
            ChannelKind::Room => {
                for c in &self.room_channels {
                    let room = Contact::Room(c);
                    let mut contact_span = Span::from(&room);
                    match self.active_channel.id {
                        Some(id) if id == c.id => {
                            contact_span.style = Style::new().fg(Color::White).bg(Color::DarkGray);
                        }
                        _ => {}
                    }
                    contacts.push(Line::from(contact_span))
                }
            }
        }

        Paragraph::new(contacts)
            .block(contacts_block)
            .render(area_contacts, buf);

        let messages_block = Block::bordered().title(title_msg).border_set(border::PLAIN);
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

        self.main_text_area.render(area_input_inner, buf);
    }
}
