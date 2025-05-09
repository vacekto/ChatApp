use crate::client_lib::util::{
    config::{THEME_SELECT, THEME_SELECT_BG},
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
                    .fg(Color::Rgb(THEME_SELECT.0, THEME_SELECT.1, THEME_SELECT.2))
                    .bg(Color::Rgb(
                        THEME_SELECT_BG.0,
                        THEME_SELECT_BG.1,
                        THEME_SELECT_BG.2,
                    ))
                    .bold(),
            );
        } else {
            return line;
        };
    }
}
