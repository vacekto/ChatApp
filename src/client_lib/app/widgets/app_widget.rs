use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    symbols::border,
    text::Line,
    widgets::{Block, Paragraph, Widget, Wrap},
};

use crate::client_lib::app::app::App;

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout_main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(area);

        let title_msg = Line::from(" Messages ");
        let title_contacts = Line::from(" Contacts ");
        let title_input = Line::from(" Input ");

        let area_left = layout_main[0].inner(Margin {
            horizontal: 2,
            vertical: 3,
        });

        let area_right = layout_main[1].inner(Margin {
            horizontal: 2,
            vertical: 3,
        });

        let layout_right = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(80), Constraint::Percentage(20)])
            .split(area_right);

        let area_messages = layout_right[0].inner(Margin {
            horizontal: 0,
            vertical: 0,
        });

        let area_input = layout_right[1].inner(Margin {
            horizontal: 0,
            vertical: 0,
        });

        let area_input_inner = area_input.inner(Margin {
            horizontal: 2,
            vertical: 2,
        });

        Block::bordered()
            .title(title_input)
            // .title_bottom(instructions.centered())
            .border_set(border::PLAIN)
            .render(area_input, buf);

        let messages_block = Block::bordered()
            .title(title_msg)
            // .title_bottom(instructions.centered())
            .border_set(border::PLAIN);
        // .render(area_messages, buf);

        Block::bordered()
            .title(title_contacts.centered())
            // .title_bottom(instructions.centered())
            .border_set(border::PLAIN)
            .render(area_left, buf);

        // let line = Line::from(my_msg);
        // let t = self.messages.clone();

        let channel = &self.active_channel.clone();
        let messages = match self.get_messages(channel) {
            Some(m) => m,
            None => &vec![],
        };

        let msg_widgets: Vec<Line> = messages.iter().map(|m| m.into()).collect();

        self.text_area.render(area_input_inner, buf);

        Paragraph::new(msg_widgets)
            .block(messages_block)
            .wrap(Wrap { trim: true })
            .render(area_messages, buf);
    }
}
