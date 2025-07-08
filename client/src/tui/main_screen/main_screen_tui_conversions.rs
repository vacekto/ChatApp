use crate::util::types::Contact;
use shared::types::{ChannelMsg, DirectChannel, ImgRender, TextMsg, TuiRoom, User};

use ratatui::{
    style::Stylize,
    text::{Line, Span, Text},
};

pub struct LineWrapper(pub Line<'static>);
pub struct TextWrapper(pub Text<'static>);

impl From<LineWrapper> for Line<'static> {
    fn from(value: LineWrapper) -> Self {
        value.0
    }
}

impl From<&ChannelMsg> for LineWrapper {
    fn from(msg: &ChannelMsg) -> Self {
        match msg {
            ChannelMsg::JoinNotification(notification) => LineWrapper::from(notification),
            ChannelMsg::TextMsg(msg) => LineWrapper::from(msg),
            ChannelMsg::Img(_) => todo!(),
        }
    }
}

impl From<&ImgRender> for TextWrapper {
    fn from(img: &ImgRender) -> Self {
        let text = Span::from(img.cache.clone());
        TextWrapper(Text::from(text))
    }
}

impl From<&TextMsg> for LineWrapper {
    fn from(msg: &TextMsg) -> Self {
        let text = Span::from(msg.text.clone());
        let username = Span::from(msg.from.username.clone() + ": ").bold();
        LineWrapper(Line::from(vec![username, text]))
    }
}

impl From<&User> for LineWrapper {
    fn from(user: &User) -> Self {
        let username = user.username.clone();
        let notification = Span::from(username + ": joined the room");
        LineWrapper(Line::from(notification))
    }
}
impl From<&TuiRoom> for LineWrapper {
    fn from(c: &TuiRoom) -> Self {
        LineWrapper(Line::from(c.name.clone()))
    }
}

impl From<&DirectChannel> for LineWrapper {
    fn from(c: &DirectChannel) -> Self {
        LineWrapper(Line::from(c.user.username.clone()))
    }
}

impl<'a> From<&Contact<'a>> for LineWrapper {
    fn from(c: &Contact<'a>) -> Self {
        match c {
            Contact::Direct(d) => LineWrapper::from(*d),
            Contact::Room(r) => LineWrapper::from(*r),
        }
    }
}
