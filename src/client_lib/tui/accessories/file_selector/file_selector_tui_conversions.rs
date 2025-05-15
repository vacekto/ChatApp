use crate::client_lib::util::{
    config::{THEME_YELLOW_1, THEME_YELLOW_2},
    types::SelectorEntry,
};
use ratatui::{
    style::{Color, Style, Stylize},
    text::Line,
};

impl From<&SelectorEntry> for Line<'static> {
    fn from(s: &SelectorEntry) -> Self {
        let line = Line::from(s.name.clone());

        if s.selected {
            return line.style(
                Style::default()
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
                    .bold(),
            );
        } else {
            return line;
        };
    }
}
