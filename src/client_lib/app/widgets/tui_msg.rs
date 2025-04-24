use ratatui::{
    style::Stylize,
    text::{Line, Span},
};

use crate::shared_lib::types::{ChannelMsg, RoomJoinNotification, TextMsg};

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
        // let sender = Span::from(msg.from);
        Line::from(vec![username, text])
    }
}

impl From<&RoomJoinNotification> for Line<'static> {
    fn from(not: &RoomJoinNotification) -> Self {
        let username = not.user.username.clone();
        let notification = Span::from(username + ": joined the room");
        Line::from(notification)
    }
}
