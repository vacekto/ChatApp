use crate::client_lib::util::types::SelectorEntry;
use ratatui::{
    style::{Color, Style},
    text::Line,
};

impl From<&SelectorEntry> for Line<'static> {
    fn from(s: &SelectorEntry) -> Self {
        let line = Line::from(s.name.clone());
        if s.selected {
            return line.style(Style::default().fg(Color::White).bg(Color::DarkGray));
        } else {
            return line;
        };
    }
}
