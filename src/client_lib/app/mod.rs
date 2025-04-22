use std::io;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    symbols::border,
    text::Line,
    widgets::{Block, Widget},
    DefaultTerminal, Frame,
};
#[derive(Debug, Default)]
pub struct App {
    counter: u8,
    exit: bool,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Left => self.decrement_counter(),
            KeyCode::Right => self.increment_counter(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn increment_counter(&mut self) {
        self.counter += 1;
    }

    fn decrement_counter(&mut self) {
        self.counter -= 1;
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout_main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(area);

        let title_msg = Line::from(" Messages ");
        let title_contacts = Line::from(" Contacts ");
        let title_input = Line::from(" Input ");

        let area_left = layout_main[0].inner(ratatui::layout::Margin {
            horizontal: 2,
            vertical: 3,
        });

        let area_right = layout_main[1].inner(ratatui::layout::Margin {
            horizontal: 2,
            vertical: 3,
        });

        let layout_right = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(80), Constraint::Percentage(20)])
            .split(area_right);

        let area_messages = layout_right[0].inner(ratatui::layout::Margin {
            horizontal: 0,
            vertical: 0,
        });

        let area_input = layout_right[1].inner(ratatui::layout::Margin {
            horizontal: 0,
            vertical: 0,
        });

        Block::bordered()
            .title(title_input)
            // .title_bottom(instructions.centered())
            .border_set(border::PLAIN)
            .render(area_input, buf);

        Block::bordered()
            .title(title_msg)
            // .title_bottom(instructions.centered())
            .border_set(border::PLAIN)
            .render(area_messages, buf);

        Block::bordered()
            .title(title_contacts.centered())
            // .title_bottom(instructions.centered())
            .border_set(border::PLAIN)
            .render(area_left, buf);
    }
}

pub fn ratatui() -> Result<()> {
    let mut terminal = ratatui::init();
    App::default().run(&mut terminal)?;
    ratatui::restore();
    Ok(())
}

// let instructions = Line::from(vec![
//     " Decrement ".into(),
//     "<Left>".blue().bold(),
//     " Increment ".into(),
//     "<Right>".blue().bold(),
//     " Quit ".into(),
//     "<Q> ".blue().bold(),
// ]);

// let counter_text = Text::from(vec![Line::from(vec![
//     "Value: ".into(),
//     self.counter.to_string().yellow(),
// ])]);

// Paragraph::new(counter_text)
//     // .centered()
//     .block(block)
//     .render(layout[1], buf);
