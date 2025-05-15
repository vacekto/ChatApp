use crate::{
    client_lib::{
        global_states::thread_logger::get_thread_runner,
        tui::app::app::App,
        util::{
            config::TCP_CHUNK_BUFFER_SIZE,
            types::{ChannelKind, SelectorEntryKind},
        },
    },
    shared_lib::types::{Channel, Chunk, ClientServerMsg, FileMetadata, User},
};
use anyhow::Result;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{io::Read, os::linux::fs::MetadataExt, path::PathBuf};
use uuid::Uuid;

impl App {
    pub fn handle_file_selector_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.exit()
            }
            KeyCode::Char('f') | KeyCode::Char('F')
                if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.close_file_selector()?
            }
            KeyCode::Esc => self.close_file_selector()?,
            KeyCode::Up => self.file_selector.move_up()?,
            KeyCode::Down => self.file_selector.move_down()?,
            KeyCode::Left => self.file_selector.close_current_folder()?,
            KeyCode::Right => self.file_selector.open_folder()?,
            KeyCode::Enter => self.handle_file_selector_enter()?,
            _ => {
                self.login_text_area.input(key_event);
            }
        };

        Ok(())
    }

    fn close_file_selector(&mut self) -> Result<()> {
        self.display_file_selector = false;
        self.file_selector.reset_location()?;
        Ok(())
    }

    pub fn handle_file_selector_enter(&mut self) -> Result<()> {
        let selected = &self.file_selector.entries[self.file_selector.selected_index];

        match selected.kind {
            SelectorEntryKind::File => {
                self.file_selector.current_location.push(&selected.name);
                let path = self.file_selector.current_location.clone();

                self.send_file(path);

                self.file_selector.reset_location()?;
                self.display_file_selector = false;
            }
            SelectorEntryKind::Folder => {
                match selected.name == "../" {
                    true => self.file_selector.close_current_folder()?,
                    false => self.file_selector.open_folder()?,
                };
            }
        }

        Ok(())
    }

    fn send_file(&mut self, path: PathBuf) {
        let id_to = match self.active_channel.id {
            None => return,
            Some(id) => id,
        };

        let tx_tui_tcp_msg = self.tx_tui_tcp_msg.clone();
        let tx_tui_tcp_file = self.tx_tui_tcp_file.clone();

        let id_from = self.id;
        let username = self.username.clone();

        let to = match self.active_channel.kind {
            ChannelKind::Direct => Channel::User(id_to),
            ChannelKind::Room => Channel::Room(id_to),
        };

        let from = match self.active_channel.kind {
            ChannelKind::Direct => Channel::User(id_from),
            ChannelKind::Room => Channel::Room(id_to),
        };

        let th_runner = get_thread_runner();

        th_runner.spawn("file transmitter", false, move || {
            let mut file = std::fs::File::open(&path)?;
            let meta = file.metadata()?;
            let stream_id = Uuid::new_v4();
            let mut buffer = [0u8; TCP_CHUNK_BUFFER_SIZE];

            let meta = FileMetadata {
                filename: String::from(path.file_name().unwrap().to_str().unwrap()),
                stream_id,
                to: to.clone(),
                size: meta.st_size(),
                from,
            };

            let metadata = ClientServerMsg::FileMetadata(meta);
            tx_tui_tcp_msg.send(metadata)?;

            loop {
                let n = file.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                let chunk = Chunk {
                    data: buffer.clone(),
                    from: User {
                        id: id_from,
                        username: username.clone(),
                    },
                    to: to.clone(),
                    stream_id,
                };
                tx_tui_tcp_file.send(chunk)?;
            }
            Ok(())
        });
    }
}
