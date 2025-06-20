use crate::{
    client_lib::{
        global_states::{console_logger::console_log, thread_logger::get_thread_runner},
        tui::app::app::App,
        util::{
            config::TCP_CHUNK_BUFFER_SIZE,
            types::{ChannelKind, FileAction, ImgRender, SelectorEntryKind},
        },
    },
    shared_lib::types::{Channel, Chunk, ClientServerMsg, FileMetadata, User},
};
use anyhow::Result;
use image::imageops::FilterType;
use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use std::{io::Read, os::linux::fs::MetadataExt, path::PathBuf};
use uuid::Uuid;

impl App {
    pub fn handle_file_selector_key_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.exit()
                    }
                    KeyCode::Char('F') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.close_file_selector()?
                    }
                    KeyCode::Char('f') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.close_file_selector()?
                    }

                    KeyCode::Char('r') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.display_room_creator = true;
                        self.display_file_selector = false;
                    }

                    KeyCode::Char('R') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.display_room_creator = true;
                        self.display_file_selector = false;
                    }
                    KeyCode::Tab => self.file_selector.switch_action()?,
                    KeyCode::Esc => self.close_file_selector()?,
                    KeyCode::Up => self.file_selector.move_up()?,
                    KeyCode::Down => self.file_selector.move_down()?,
                    KeyCode::Left => self.file_selector.close_current_folder()?,
                    KeyCode::Right => self.file_selector.open_folder()?,
                    KeyCode::Enter => self.handle_file_selector_enter()?,
                    _ => {
                        self.username_ta_login.input(key_event);
                    }
                };
            }
            _ => {}
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

        match (&selected.kind, &self.file_selector.active_action) {
            (SelectorEntryKind::File, FileAction::File) => {
                self.file_selector.current_location.push(&selected.name);
                let path = self.file_selector.current_location.clone();

                self.send_file(path);

                self.file_selector.reset_location()?;
                self.display_file_selector = false;
            }
            (SelectorEntryKind::File, FileAction::ASCII) => {
                let to = match (&self.active_channel.kind, &self.active_channel.id) {
                    (_, None) => return Ok(()),
                    (ChannelKind::Direct, Some(id)) => Channel::User(*id),
                    (ChannelKind::Room, Some(id)) => Channel::Room(*id),
                };

                let from = User {
                    username: self.username.clone(),
                    id: self.id,
                };

                let th = get_thread_runner();
                let path = format!(
                    "{}/{}",
                    self.file_selector
                        .current_location
                        .clone()
                        .to_str()
                        .unwrap(),
                    selected.name
                );
                let tx_tui_tcp_msg = self.tx_tui_tcp_msg.clone();

                th.spawn("image to ascii converter", false, move || {
                    console_log(&format!("{path:?}"));
                    let image = image::open(path).expect("Failed to open image");
                    let resized = image.resize_exact(50, 70, FilterType::Nearest);
                    let conf = artem::config::ConfigBuilder::new().color(false).build();
                    let ascii = artem::convert(resized, &conf);
                    let img_render = ImgRender {
                        cache: ascii,
                        from,
                        to,
                    };
                    let msg = ClientServerMsg::ASCII(img_render);
                    tx_tui_tcp_msg.send(msg).unwrap();

                    Ok(())
                });
            }

            (SelectorEntryKind::Folder, _) => {
                if selected.name == "../" {
                    self.file_selector.close_current_folder()?;
                } else {
                    self.file_selector.open_folder()?;
                }
            }
        };

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
