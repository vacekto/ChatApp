use ratatui::{style::Stylize, text::Span};

use crate::{
    client_lib::util::types::Contact,
    shared_lib::types::{DirectChannel, RoomChannel},
};

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
