use std::{collections::HashMap, str::FromStr, sync::mpsc};

use anyhow::Result;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    DefaultTerminal, Frame,
};
use tui_textarea::TextArea;
use uuid::Uuid;

use crate::{
    client_lib::{
        global_states::thread_logger::get_thread_runner,
        util::types::{ActiveChannel, ActiveStream, ChannelKind, TuiUpdate},
    },
    shared_lib::{
        config::PUBLIC_ROOM_ID,
        types::{
            Channel, ChannelMsg, ClientServerMsg, DirectChannel, InitClientData, RoomChannel,
            ServerClientMsg, TextMsg, User,
        },
    },
};

use super::app_functions::{handle_file_chunk, handle_file_metadata};

pub struct App {
    pub username: String,
    pub id: Uuid,
    pub exit: bool,
    pub text_area: TextArea<'static>,
    pub tx_tui_write: mpsc::Sender<ClientServerMsg>,
    pub room_channels: Vec<RoomChannel>,
    pub direct_channels: Vec<DirectChannel>,
    pub selected_channel: ActiveChannel,
    // pub active_channel: ActiveChannel,
    pub _active_streams: HashMap<Uuid, ActiveStream>,
}

impl App {
    pub fn new(tx_tui_write: mpsc::Sender<ClientServerMsg>, init_data: InitClientData) -> Self {
        App {
            username: init_data.username,
            id: init_data.id,
            exit: false,
            text_area: TextArea::default(),
            tx_tui_write,
            selected_channel: ActiveChannel {
                id: None,
                kind: ChannelKind::Room,
            },
            direct_channels: vec![],
            room_channels: vec![],
            _active_streams: HashMap::new(),
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        rx_client_tui: mpsc::Receiver<ServerClientMsg>,
    ) -> Result<()> {
        let (tx_tui, rx_tui) = mpsc::channel::<TuiUpdate>();

        self.listen_for_server(rx_client_tui, tx_tui.clone())?;
        self.listen_for_events(tx_tui.clone())?;

        while !self.exit {
            match rx_tui.recv()? {
                TuiUpdate::Event(e) => self.handle_events(e)?,
                TuiUpdate::ServerMsg(msg) => self.handle_server_msg(msg)?,
            }
            terminal.draw(|frame| self.draw(frame))?;
        }
        Ok(())
    }

    fn listen_for_events(&self, tx_tui: mpsc::Sender<TuiUpdate>) -> Result<()> {
        let th_runner = get_thread_runner();
        th_runner.run("client_tui_listener", move || loop {
            let e = event::read()?;
            tx_tui.send(TuiUpdate::Event(e))?
        });

        Ok(())
    }

    fn listen_for_server(
        &self,
        rx_server_tui: mpsc::Receiver<ServerClientMsg>,
        tx_tui: mpsc::Sender<TuiUpdate>,
    ) -> Result<()> {
        let th_runner = get_thread_runner();

        th_runner.run("server_client_listener", move || {
            while let Ok(msg) = rx_server_tui.recv() {
                tx_tui.send(TuiUpdate::ServerMsg(msg))?;
            }

            Ok(())
        });
        Ok(())
    }

    fn handle_server_msg(&mut self, msg: ServerClientMsg) -> Result<()> {
        match msg {
            ServerClientMsg::FileChunk(chunk) => handle_file_chunk(chunk)?,
            ServerClientMsg::FileMetadata(meta) => handle_file_metadata(meta)?,
            ServerClientMsg::Text(msg) => self.handle_text_message(msg),
            ServerClientMsg::RoomUpdate(room) => self.handle_room_update(room),
            ServerClientMsg::JoinRoom(room) => self.handle_room_invitation(room),
        };

        Ok(())
    }

    fn handle_room_invitation(&mut self, mut room: RoomChannel) {
        if room.id == Uuid::from_str(PUBLIC_ROOM_ID).unwrap() {
            if let Some(i) = room.users.iter().position(|u| u.id == self.id) {
                room.users.remove(i);
            };

            for user in &room.users {
                self.direct_channels.push(DirectChannel {
                    messages: vec![],
                    user: (*user).clone(),
                });
            }
        };

        self.room_channels.push(room);
    }

    fn handle_room_update(&mut self, mut room: RoomChannel) {
        if room.id == Uuid::from_str(PUBLIC_ROOM_ID).unwrap() {
            if let Some(i) = room.users.iter().position(|u| u.id == self.id) {
                room.users.remove(i);
            };

            let mut direct_channels_update: Vec<DirectChannel> = vec![];

            for user in room.users.clone() {
                if let Some(i) = self
                    .direct_channels
                    .iter()
                    .position(|c| c.user.id == user.id)
                {
                    direct_channels_update.push(self.direct_channels[i].clone());
                } else {
                    direct_channels_update.push(DirectChannel {
                        messages: vec![],
                        user: (user).clone(),
                    });
                };
            }

            self.direct_channels = direct_channels_update;

            match (&self.selected_channel.kind, self.selected_channel.id) {
                (ChannelKind::Direct, Some(id)) => {
                    if !self.direct_channels.iter().any(|c| c.user.id == id) {
                        self.selected_channel.id = None;
                    }
                }
                (ChannelKind::Room, Some(id)) => {
                    if !self.room_channels.iter().any(|c| c.id == id) {
                        self.selected_channel.id = None;
                    }
                }
                _ => {}
            }
        };

        if let Some(index) = self.room_channels.iter().position(|c| c.id == room.id) {
            self.room_channels[index] = room;
        };
    }

    pub fn handle_text_message(&mut self, msg: TextMsg) {
        match msg.to {
            Channel::Room(id) => {
                if let Some(messages) = self.get_channel_messages(id) {
                    messages.push(ChannelMsg::TextMsg(msg))
                };
            }
            Channel::User(_) => {
                if let Some(messages) = self.get_direct_messages(msg.from.id) {
                    messages.push(ChannelMsg::TextMsg(msg));
                }
            }
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn get_direct_messages(&mut self, id: Uuid) -> Option<&mut Vec<ChannelMsg>> {
        let res = self.direct_channels.iter_mut().find(|c| c.user.id == id);
        match res {
            Some(c) => Some(&mut c.messages),
            None => None,
        }
    }

    fn get_channel_messages(&mut self, id: Uuid) -> Option<&mut Vec<ChannelMsg>> {
        let res = self.room_channels.iter_mut().find(|c| c.id == id);
        match res {
            Some(c) => Some(&mut c.messages),
            None => None,
        }
    }

    fn handle_events(&mut self, e: Event) -> Result<()> {
        match e {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)?
            }

            _ => {}
        };

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Enter => self.send_message()?,
            KeyCode::Up => self.move_active_channel_up(),
            KeyCode::Down => self.move_active_channel_down(),
            KeyCode::Left => self.switch_contacts(),
            KeyCode::Right => self.switch_contacts(),
            _ => {
                self.text_area.input(key_event);
            }
        };

        Ok(())
    }

    fn switch_contacts(&mut self) {
        let new_kind = match self.selected_channel.kind {
            ChannelKind::Direct => ChannelKind::Room,
            ChannelKind::Room => ChannelKind::Direct,
        };

        let new_id = match new_kind {
            ChannelKind::Direct => {
                if self.direct_channels.len() == 0 {
                    None
                } else {
                    Some(self.direct_channels[0].user.id)
                }
            }
            ChannelKind::Room => {
                if self.room_channels.len() == 0 {
                    None
                } else {
                    Some(self.room_channels[0].id)
                }
            }
        };

        self.selected_channel.id = new_id;
        self.selected_channel.kind = new_kind;
    }

    fn move_active_channel_up(&mut self) {
        match (&self.selected_channel.kind, self.selected_channel.id) {
            (ChannelKind::Direct, Some(id)) => {
                let index = self.direct_channels.iter().position(|c| c.user.id == id);
                match index {
                    None => panic!("active channel id not in state"),
                    Some(i) => {
                        if i == 0 {
                            return;
                        }
                        self.selected_channel.id = Some(self.direct_channels[i - 1].user.id)
                    }
                }
            }
            (ChannelKind::Room, Some(id)) => {
                let index = self.room_channels.iter().position(|c| c.id == id);
                match index {
                    None => panic!("active channel id not in state"),
                    Some(i) => {
                        if i == 0 {
                            return;
                        }
                        self.selected_channel.id = Some(self.room_channels[i - 1].id)
                    }
                }
            }
            (ChannelKind::Direct, None) => {
                if self.direct_channels.len() == 0 {
                    return;
                }
                self.selected_channel.id = Some(self.direct_channels[0].user.id);
            }
            (ChannelKind::Room, None) => {
                if self.room_channels.len() == 0 {
                    return;
                }
                self.selected_channel.id = Some(self.room_channels[0].id);
            }
        }
    }

    fn move_active_channel_down(&mut self) {
        match (&self.selected_channel.kind, self.selected_channel.id) {
            (ChannelKind::Direct, Some(id)) => {
                let index = self.direct_channels.iter().position(|c| c.user.id == id);
                match index {
                    None => panic!("active channel id not in state"),
                    Some(i) => {
                        if i == self.direct_channels.len() - 1 {
                            return;
                        }
                        self.selected_channel.id = Some(self.direct_channels[i + 1].user.id)
                    }
                }
            }
            (ChannelKind::Room, Some(id)) => {
                let index = self.room_channels.iter().position(|c| c.id == id);
                match index {
                    None => panic!("active channel id not in state"),
                    Some(i) => {
                        if i == self.room_channels.len() - 1 {
                            return;
                        }
                        self.selected_channel.id = Some(self.room_channels[i + 1].id)
                    }
                }
            }
            (ChannelKind::Direct, None) => {
                if self.direct_channels.len() == 0 {
                    return;
                }
                self.selected_channel.id = Some(self.direct_channels[0].user.id);
            }
            (ChannelKind::Room, None) => {
                if self.room_channels.len() == 0 {
                    return;
                }
                self.selected_channel.id = Some(self.room_channels[0].id);
            }
        }
    }

    fn send_message(&mut self) -> Result<()> {
        let id = match self.selected_channel.id {
            None => return Ok(()),
            Some(id) => id,
        };
        let text = self.text_area.lines().join("\n");

        let from = User {
            username: self.username.clone(),
            id: self.id,
        };

        let to = match self.selected_channel.kind {
            ChannelKind::Direct => Channel::User(id),
            ChannelKind::Room => Channel::Room(id),
        };

        let msg = TextMsg { text, from, to };

        if let Some(messages) = self.get_direct_messages(id) {
            messages.push(ChannelMsg::TextMsg(msg.clone()));
        };

        let msg = ClientServerMsg::Text(msg);

        self.tx_tui_write.send(msg)?;
        self.text_area = TextArea::default();

        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}
