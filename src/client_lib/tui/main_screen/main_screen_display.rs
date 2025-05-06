use crate::{
    client_lib::{
        tui::app::app::App,
        util::types::{ChannelKind, Contact},
    },
    shared_lib::types::{ChannelMsg, DirectChannel, RoomChannel, TextMsg, User},
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
                    let mut span = Span::from(&contact);
                    match self.active_channel.id {
                        Some(id) if id == c.user.id => {
                            span.style = Style::default().fg(Color::White).bg(Color::DarkGray);
                        }
                        _ => {}
                    }
                    contacts.push(Line::from(span))
                }
            }
            ChannelKind::Room => {
                for c in &self.room_channels {
                    let room = Contact::Room(c);
                    let mut span = Span::from(&room);
                    match self.active_channel.id {
                        Some(id) if id == c.id => {
                            span.style = Style::new().fg(Color::White).bg(Color::DarkGray);
                        }
                        _ => {}
                    }
                    contacts.push(Line::from(span))
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

impl From<&ChannelMsg> for Line<'static> {
    fn from(msg: &ChannelMsg) -> Self {
        match msg {
            ChannelMsg::JoinNotification(notification) => Line::from(notification),
            ChannelMsg::TextMsg(msg) => Line::from(msg),
        }
    }
}

impl From<&TextMsg> for Line<'static> {
    fn from(msg: &TextMsg) -> Self {
        let text = Span::from(msg.text.clone());
        let username = Span::from(msg.from.username.clone() + ": ").bold();
        Line::from(vec![username, text])
    }
}

impl From<&User> for Line<'static> {
    fn from(user: &User) -> Self {
        let username = user.username.clone();
        let notification = Span::from(username + ": joined the room");
        Line::from(notification)
    }
}

impl From<&RoomChannel> for Span<'_> {
    fn from(c: &RoomChannel) -> Self {
        Span::from(c.name.clone()).bold()
    }
}

impl From<&DirectChannel> for Span<'_> {
    fn from(c: &DirectChannel) -> Self {
        Span::from(c.user.username.clone())
    }
}

impl<'a> From<&Contact<'a>> for Span<'a> {
    fn from(c: &Contact<'a>) -> Self {
        match c {
            Contact::Direct(d) => Span::from(*d),
            Contact::Room(r) => Span::from(*r),
        }
    }
}
