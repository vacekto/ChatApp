use crate::{
    client_lib::{
        global_states::{app_state::get_global_state, console_logger::console_log},
        util::{
            config::FILES_DIR,
            types::{
                ActiveChannel, ActiveScreen, ActiveStream, ChannelKind, FileSelector, TuiUpdate,
            },
        },
    },
    shared_lib::types::{
        Channel, ChannelMsg, Chunk, DirectChannel, FileMetadata, InitClientData, RoomChannel,
        TextMsg, TuiServerMsg, User,
    },
};
use anyhow::Result;
use ratatui::{
    crossterm::event::{Event, KeyEventKind},
    layout::Margin,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
    DefaultTerminal, Frame,
};
use std::{
    collections::HashMap,
    io::Write,
    path::Path,
    sync::mpsc::{self},
};
use tui_textarea::TextArea;
use uuid::Uuid;

pub struct App {
    pub username: String,
    pub id: Uuid,
    pub exit: bool,
    pub login_text_area: TextArea<'static>,
    pub main_text_area: TextArea<'static>,
    pub tx_tui_tcp: mpsc::Sender<TuiServerMsg>,
    pub room_channels: Vec<RoomChannel>,
    pub direct_channels: Vec<DirectChannel>,
    pub active_channel: ActiveChannel,
    pub active_streams: HashMap<Uuid, ActiveStream>,
    pub active_screen: ActiveScreen,
    pub display_file_selector: bool,
    pub file_selector: FileSelector,
    pub login_notification: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let state = get_global_state();
        let tx_tui_tcp = state.tui_tcp_channel.tx.clone();
        drop(state);

        App {
            username: String::new(),
            id: Uuid::nil(),
            exit: false,
            login_text_area: TextArea::default(),
            main_text_area: TextArea::default(),
            active_channel: ActiveChannel {
                id: None,
                kind: ChannelKind::Room,
            },
            tx_tui_tcp,
            direct_channels: vec![],
            room_channels: vec![],
            active_streams: HashMap::new(),
            active_screen: ActiveScreen::Login,
            display_file_selector: false,
            file_selector: FileSelector::new(),
            login_notification: None,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let (tx_tui, rx_tui) = mpsc::channel::<TuiUpdate>();

        self.listen_for_server(tx_tui.clone())?;
        self.listen_for_events(tx_tui.clone())?;

        while !self.exit {
            console_log(&format!("{}", self.file_selector.selected_index));
            terminal.draw(|frame| self.draw(frame))?;
            match rx_tui.recv()? {
                TuiUpdate::Event(e) => self.handle_events(e)?,
                TuiUpdate::ServerMsg(msg) => self.handle_server_msg(msg)?,
            }
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        if self.display_file_selector {
            frame.render_widget(&mut self.file_selector, frame.area());
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(None);

            let mut scrollbar_state = ScrollbarState::new(self.file_selector.entries.len())
                .position(self.file_selector.scroll_offset as usize);

            let area = frame.area();
            frame.render_stateful_widget(
                scrollbar,
                area.inner(Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut scrollbar_state,
            );
        }
        frame.render_widget(self, frame.area());
    }

    pub fn init(&mut self, init: InitClientData) {
        self.username = init.username;
        self.id = init.id;
    }

    pub fn logout(&mut self) -> Result<()> {
        let msg = TuiServerMsg::Logout;
        self.tx_tui_tcp.send(msg)?;
        self.active_screen = ActiveScreen::Login;
        self.direct_channels = vec![];
        self.room_channels = vec![];

        Ok(())
    }

    fn handle_events(&mut self, e: Event) -> Result<()> {
        match (&self.active_screen, self.display_file_selector) {
            (_, true) => {
                match e {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        self.handle_file_selector_key_event(key_event)?
                    }
                    _ => {}
                };
            }
            (ActiveScreen::Login, false) => {
                match e {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        self.handle_login_key_event(key_event)?
                    }
                    _ => {}
                };
            }
            (ActiveScreen::Main, false) => {
                match e {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        self.handle_main_key_event(key_event)?
                    }
                    _ => {}
                };
            }
        }

        Ok(())
    }
    pub fn handle_file_metadata(&mut self, meta: FileMetadata) -> Result<()> {
        let path = String::from(FILES_DIR) + &meta.name;
        let path = Path::new(&path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = std::fs::File::create(path)?;
        let stream_id = meta.stream_id;

        let stream = ActiveStream {
            file_handle: file,
            size: meta.size,
            written: 0,
        };

        self.active_streams.insert(stream_id, stream);

        Ok(())
    }

    pub fn handle_file_chunk(&mut self, chunk: Chunk) -> Result<()> {
        let stream = self.active_streams.get_mut(&chunk.stream_id).unwrap();
        let bytes_to_write =
            std::cmp::min(chunk.data.len(), (stream.size - stream.written) as usize);

        stream
            .file_handle
            .write_all(&chunk.data[0..bytes_to_write])?;
        stream.written += chunk.data.len() as u64;

        let written = stream.written;
        let size = stream.size;

        if written == size {
            self.active_streams.remove(&chunk.stream_id).unwrap();
        }

        Ok(())
    }

    pub fn send_message(&mut self) -> Result<()> {
        let id = match self.active_channel.id {
            None => return Ok(()),
            Some(id) => id,
        };
        let text = self.main_text_area.lines().join("\n");

        let from = User {
            username: self.username.clone(),
            id: self.id,
        };

        let to = match self.active_channel.kind {
            ChannelKind::Direct => Channel::User(id),
            ChannelKind::Room => Channel::Room(id),
        };

        let msg = TextMsg { text, from, to };

        if let Some(messages) = self.get_direct_messages(id) {
            messages.push(ChannelMsg::TextMsg(msg.clone()));
        };

        let msg = TuiServerMsg::Text(msg);

        self.tx_tui_tcp.send(msg)?;
        self.main_text_area = TextArea::default();

        Ok(())
    }

    pub fn exit(&mut self) {
        self.exit = true;
    }
}
