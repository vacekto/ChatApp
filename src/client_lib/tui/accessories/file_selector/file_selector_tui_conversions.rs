use crate::client_lib::util::{
    config::{THEME_YELLOW_DARK, THEME_YELLOW_LIGHT},
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
                        THEME_YELLOW_DARK.0,
                        THEME_YELLOW_DARK.1,
                        THEME_YELLOW_DARK.2,
                    ))
                    .bg(Color::Rgb(
                        THEME_YELLOW_LIGHT.0,
                        THEME_YELLOW_LIGHT.1,
                        THEME_YELLOW_LIGHT.2,
                    ))
                    .bold(),
            );
        } else {
            return line;
        };
    }
}
