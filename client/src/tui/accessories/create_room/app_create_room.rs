use crate::{
    tui::app::app::App,
    util::types::{ActiveCreateRoomInput, RoomAction},
};
use anyhow::Result;
use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use shared::types::{ClientServerMsg, RoomUpdateTransit};

impl App {
    pub async fn handle_create_room_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.exit()
                    }
                    KeyCode::Char('r') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.display_room_creator = false
                    }

                    KeyCode::Char('R') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.display_room_creator = false
                    }

                    KeyCode::Char('f') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.display_room_creator = false;
                        self.display_file_selector = true;
                    }

                    KeyCode::Char('F') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.display_room_creator = false;
                        self.display_file_selector = true;
                    }

                    KeyCode::Esc => self.display_room_creator = false,
                    KeyCode::Up => self.room_creator.move_active_input_up(),
                    KeyCode::Down => self.room_creator.move_active_input_down(),
                    KeyCode::Enter => self.handle_room_submit().await?,
                    KeyCode::Tab => self.room_creator.switch_action(),
                    _ => {
                        if self.room_creator.active_input == ActiveCreateRoomInput::Name {
                            self.room_creator.room_name_ta.input(key_event);
                        } else {
                            self.room_creator.room_password_ta.input(key_event);
                        }
                    }
                };
            }
            _ => {}
        };

        Ok(())
    }

    pub async fn handle_room_submit(&mut self) -> Result<()> {
        let room_name = String::from(self.room_creator.room_name_ta.lines().join("").trim());
        let room_password =
            String::from(self.room_creator.room_password_ta.lines().join("").trim());

        let room_password = if room_password.is_empty() {
            None
        } else {
            Some(room_password)
        };

        let transit = RoomUpdateTransit {
            room_name,
            room_password,
        };

        let msg = match self.room_creator.active_action {
            RoomAction::Create => ClientServerMsg::CreateRoom(transit),
            RoomAction::Join => ClientServerMsg::JoinRoom(transit),
        };
        self.tx_tui_ws_msg.send(msg).await.ok();
        Ok(())
    }
}
