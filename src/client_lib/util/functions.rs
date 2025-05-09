use ratatui::style::Style;
use ratatui::text::{Line, Span};

pub fn pad_line_to_width(mut line: Line, target_width: u16) -> Line {
    let content_width: usize = line.width();
    if content_width < target_width as usize {
        let padding = " ".repeat(target_width as usize - content_width);
        line.spans.push(Span::styled(padding, Style::default()));
    }
    line
}
