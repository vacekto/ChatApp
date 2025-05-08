use std::io::Write;

use crate::{
    client_lib::{
        global_states::app_state::get_global_state, tui::app::app::App, write_server::frame_data,
    },
    shared_lib::types::AuthData,
};
use anyhow::Result;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    pub fn handle_login_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.exit()
            }
            KeyCode::Esc => self.exit(),
            KeyCode::Enter => self.send_auth()?,
            _ => {
                self.login_text_area.input(key_event);
            }
        };

        Ok(())
    }

    fn send_auth(&self) -> Result<()> {
        let username = self.login_text_area.lines().join("\n");
        let mut state = get_global_state();

        let data = AuthData { username };

        let serialized = bincode::serialize(&data)?;
        let framed = frame_data(&serialized);
        state.tcp.write_all(&framed)?;

        Ok(())
    }
}
