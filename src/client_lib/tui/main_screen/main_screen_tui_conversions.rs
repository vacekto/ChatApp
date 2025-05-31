use crate::{
    client_lib::util::types::{Contact, ImgRender},
    shared_lib::types::{DirectChannel, TextMsg, TuiMsg, TuiRoom, User},
};

use ratatui::{
    style::Stylize,
    text::{Line, Span, Text},
};

impl From<&TuiMsg> for Line<'static> {
    fn from(msg: &TuiMsg) -> Self {
        match msg {
            TuiMsg::JoinNotification(notification) => Line::from(notification),
            TuiMsg::TextMsg(msg) => Line::from(msg),
            TuiMsg::Img(_img) => todo!(),
        }
    }
}

impl From<&ImgRender> for Text<'static> {
    fn from(img: &ImgRender) -> Self {
        let text = Span::from(img.cache.clone());
        Text::from(text)
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
impl From<&TuiRoom> for Line<'_> {
    fn from(c: &TuiRoom) -> Self {
        Line::from(c.name.clone())
    }
}

impl From<&DirectChannel> for Line<'_> {
    fn from(c: &DirectChannel) -> Self {
        Line::from(c.user.username.clone())
    }
}

impl<'a> From<&Contact<'a>> for Line<'a> {
    fn from(c: &Contact<'a>) -> Self {
        match c {
            Contact::Direct(d) => Line::from(*d),
            Contact::Room(r) => Line::from(*r),
        }
    }
}
