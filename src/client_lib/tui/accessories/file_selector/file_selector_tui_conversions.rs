use crate::client_lib::util::{
    config::{THEME_YELLOW_DARK, THEME_YELLOW_LIGHT},
    types::{SelectorEntry, SelectorEntryKind},
};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

impl From<&SelectorEntry> for Line<'static> {
    fn from(s: &SelectorEntry) -> Self {
        let yellow_light = Color::Rgb(
            THEME_YELLOW_LIGHT.0,
            THEME_YELLOW_LIGHT.1,
            THEME_YELLOW_LIGHT.2,
        );

        let yellow_dark = Color::Rgb(
            THEME_YELLOW_DARK.0,
            THEME_YELLOW_DARK.1,
            THEME_YELLOW_DARK.2,
        );

        let dark_gray = Color::Rgb(180, 180, 180);

        let style_icon = match (&s.kind, s.selected) {
            (SelectorEntryKind::File, false) => Style::new().fg(yellow_dark),
            (SelectorEntryKind::Folder, false) => Style::new().fg(yellow_light),
            (SelectorEntryKind::File, true) => Style::new().fg(yellow_dark),
            (SelectorEntryKind::Folder, true) => Style::new().fg(yellow_dark),
        };

        let style_text = match (&s.kind, s.selected) {
            (SelectorEntryKind::File, false) => Style::new().fg(Color::White),
            (SelectorEntryKind::Folder, false) => Style::new().fg(dark_gray),
            (SelectorEntryKind::File, true) => Style::new().fg(yellow_dark),
            (SelectorEntryKind::Folder, true) => Style::new().fg(Color::DarkGray),
        };

        let icon = match s.kind {
            SelectorEntryKind::File => Span::styled(" \u{ea7b}", style_icon),
            SelectorEntryKind::Folder => Span::styled(" \u{ea83}", style_icon),
        };
        let name = Span::styled(format!("  {}", s.name.clone()), style_text);
        let line = Line::from(vec![icon, name]);

        if s.selected {
            return line.style(Style::default().fg(yellow_dark).bg(yellow_light));
        } else {
            return line;
        };
    }
}
