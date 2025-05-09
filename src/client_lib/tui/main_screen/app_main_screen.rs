use crate::{
    client_lib::{
        global_states::{app_state::get_global_state, thread_logger::get_thread_runner},
        tui::app::app::App,
        util::types::{ActiveScreen, ChannelKind, TuiUpdate},
    },
    shared_lib::{
        config::PUBLIC_ROOM_ID,
        types::{
            AuthResponse, Channel, ChannelMsg, DirectChannel, RoomChannel, ServerTuiMsg, TextMsg,
        },
    },
};
use anyhow::Result;
use ratatui::crossterm::event::{self, KeyCode, KeyEvent, KeyModifiers};
use std::{
    str::FromStr,
    sync::mpsc::{self},
};
use uuid::Uuid;

impl App {
    pub fn listen_for_events(&self, tx_tui: mpsc::Sender<TuiUpdate>) -> Result<()> {
        let th_runner = get_thread_runner();

        th_runner.spawn("tui events manager", false, move || loop {
            let e = event::read()?;
            tx_tui.send(TuiUpdate::Event(e))?;
        });

        Ok(())
    }

    pub fn listen_for_server(&self, tx_tui_tui: mpsc::Sender<TuiUpdate>) -> Result<()> {
        let th_runner = get_thread_runner();

        th_runner.spawn("server messages manager", true, move || {
            let mut state = get_global_state();
            let rx = state
                .tcp_tui_channel
                .rx
                .take()
                .expect("already taken, can listen only once!!");

            drop(state);

            loop {
                let msg = rx.recv()?;
                tx_tui_tui.send(TuiUpdate::ServerMsg(msg))?;
            }
        });

        Ok(())
    }

    pub fn handle_server_msg(&mut self, msg: ServerTuiMsg) -> Result<()> {
        match msg {
            ServerTuiMsg::FileChunk(chunk) => self.handle_file_chunk(chunk)?,
            ServerTuiMsg::FileMetadata(meta) => self.handle_file_metadata(meta)?,
            ServerTuiMsg::Text(msg) => self.handle_text_message(msg),
            ServerTuiMsg::RoomUpdate(room) => self.handle_room_update(room),
            ServerTuiMsg::JoinRoom(room) => self.handle_room_invitation(room),
            ServerTuiMsg::Auth(data) => self.handle_auth_response(data),
        };

        Ok(())
    }

    fn handle_auth_response(&mut self, data: AuthResponse) {
        match data {
            AuthResponse::Failure(msg) => self.login_notification = Some(msg),
            AuthResponse::Success(init) => {
                self.username = init.username;
                self.id = init.id;
                self.active_screen = ActiveScreen::Main
            }
        }
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

            match (&self.active_channel.kind, self.active_channel.id) {
                (ChannelKind::Direct, Some(id)) => {
                    if !self.direct_channels.iter().any(|c| c.user.id == id) {
                        self.active_channel.id = None;
                    }
                }
                (ChannelKind::Room, Some(id)) => {
                    if !self.room_channels.iter().any(|c| c.id == id) {
                        self.active_channel.id = None;
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
    pub fn get_direct_messages(&mut self, id: Uuid) -> Option<&mut Vec<ChannelMsg>> {
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

    pub fn handle_main_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.exit()
            }
            KeyCode::Char('`') => self.display_file_selector = true,
            KeyCode::Enter => self.send_message()?,
            KeyCode::Esc => self.logout()?,
            KeyCode::Up => self.move_active_channel_up(),
            KeyCode::Down => self.move_active_channel_down(),
            KeyCode::Left => self.switch_channel_kind(),
            KeyCode::Right => self.switch_channel_kind(),
            _ => {
                self.main_text_area.input(key_event);
            }
        };

        Ok(())
    }

    pub fn switch_channel_kind(&mut self) {
        let new_kind = match self.active_channel.kind {
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

        self.active_channel.id = new_id;
        self.active_channel.kind = new_kind;
    }

    pub fn move_active_channel_up(&mut self) {
        match (&self.active_channel.kind, self.active_channel.id) {
            (ChannelKind::Direct, Some(id)) => {
                let index = self.direct_channels.iter().position(|c| c.user.id == id);
                match index {
                    None => panic!("active channel id not in state"),
                    Some(i) => {
                        if i == 0 {
                            return;
                        }
                        self.active_channel.id = Some(self.direct_channels[i - 1].user.id)
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
                        self.active_channel.id = Some(self.room_channels[i - 1].id)
                    }
                }
            }
            (ChannelKind::Direct, None) => {
                if self.direct_channels.len() == 0 {
                    return;
                }
                self.active_channel.id = Some(self.direct_channels[0].user.id);
            }
            (ChannelKind::Room, None) => {
                if self.room_channels.len() == 0 {
                    return;
                }
                self.active_channel.id = Some(self.room_channels[0].id);
            }
        }
    }

    pub fn move_active_channel_down(&mut self) {
        match (&self.active_channel.kind, self.active_channel.id) {
            (ChannelKind::Direct, Some(id)) => {
                let index = self.direct_channels.iter().position(|c| c.user.id == id);
                match index {
                    None => panic!("active channel id not in state"),
                    Some(i) => {
                        if i == self.direct_channels.len() - 1 {
                            return;
                        }
                        self.active_channel.id = Some(self.direct_channels[i + 1].user.id)
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
                        self.active_channel.id = Some(self.room_channels[i + 1].id)
                    }
                }
            }
            (ChannelKind::Direct, None) => {
                if self.direct_channels.len() == 0 {
                    return;
                }
                self.active_channel.id = Some(self.direct_channels[0].user.id);
            }
            (ChannelKind::Room, None) => {
                if self.room_channels.len() == 0 {
                    return;
                }
                self.active_channel.id = Some(self.room_channels[0].id);
            }
        }
    }
}
