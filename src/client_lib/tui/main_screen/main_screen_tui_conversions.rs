use crate::{
    client_lib::util::types::Contact,
    shared_lib::types::{ChannelMsg, DirectChannel, RoomChannel, TextMsg, User},
};

use ratatui::{
    style::Stylize,
    text::{Line, Span},
};

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
