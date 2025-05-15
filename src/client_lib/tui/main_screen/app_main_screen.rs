use crate::{
    client_lib::{
        tui::app::app::App,
        util::{
            config::MESSAGES_SCROLL_RESERVE,
            types::{ActiveScreen, ChannelKind, Focus},
        },
    },
    shared_lib::{
        config::PUBLIC_ROOM_ID,
        types::{AuthResponse, Channel, DirectChannel, RoomChannel, TextMsg, TuiMsg},
    },
};
use anyhow::Result;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{collections::VecDeque, str::FromStr};
use uuid::Uuid;

impl App {
    pub fn handle_auth_response(&mut self, data: AuthResponse) {
        match data {
            AuthResponse::Failure(msg) => self.login_notification = Some(msg),
            AuthResponse::Success(init) => {
                self.username = init.username;
                self.id = init.id;
                self.active_screen = ActiveScreen::Main
            }
        }
    }

    pub fn handle_room_invitation(&mut self, mut room: RoomChannel) {
        if room.id == Uuid::from_str(PUBLIC_ROOM_ID).unwrap() {
            if let Some(i) = room.users.iter().position(|u| u.id == self.id) {
                room.users.remove(i);
            };

            for user in &room.users {
                self.direct_channels.push(DirectChannel {
                    messages: VecDeque::new(),
                    user: (*user).clone(),
                });
            }
        };

        self.room_channels.push(room);
    }

    pub fn handle_room_update(&mut self, mut room: RoomChannel) {
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
                        messages: VecDeque::new(),
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
                if let Some(messages) = self.get_room_messages(id) {
                    messages.push_front(TuiMsg::TextMsg(msg))
                };
            }
            Channel::User(_) => {
                if let Some(messages) = self.get_direct_messages(msg.from.id) {
                    messages.push_front(TuiMsg::TextMsg(msg));
                }
            }
        }
    }
    pub fn get_direct_messages(&mut self, id: Uuid) -> Option<&mut VecDeque<TuiMsg>> {
        let res = self.direct_channels.iter_mut().find(|c| c.user.id == id);
        match res {
            Some(c) => Some(&mut c.messages),
            None => None,
        }
    }

    pub fn get_room_messages(&mut self, id: Uuid) -> Option<&mut VecDeque<TuiMsg>> {
        let res = self.room_channels.iter_mut().find(|c| c.id == id);
        match res {
            Some(c) => Some(&mut c.messages),
            None => None,
        }
    }

    pub fn handle_main_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match self.focus {
            Focus::Contacts => self.handle_contacts_event(key_event)?,
            Focus::Messages => self.handle_messages_event(key_event)?,
        };

        Ok(())
    }

    fn handle_contacts_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.exit()
            }
            KeyCode::Left | KeyCode::Right if key_event.modifiers.contains(KeyModifiers::SHIFT) => {
                self.switch_focus()
            }
            KeyCode::Char('f') | KeyCode::Char('F')
                if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.display_file_selector = true
            }
            KeyCode::Esc => self.logout()?,
            KeyCode::Up => self.move_active_channel_up(),
            KeyCode::Down => self.move_active_channel_down(),
            KeyCode::Left => self.switch_channel_kind(),
            KeyCode::Right => self.switch_channel_kind(),
            KeyCode::Enter => self.send_message()?,
            _ => {
                self.main_text_area.input(key_event);
            }
        };
        Ok(())
    }

    fn handle_messages_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.exit()
            }
            KeyCode::Left | KeyCode::Right if key_event.modifiers.contains(KeyModifiers::SHIFT) => {
                self.switch_focus()
            }
            KeyCode::Char('f') | KeyCode::Char('F')
                if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.display_file_selector = true
            }
            KeyCode::Enter => self.send_message()?,
            KeyCode::Esc => self.logout()?,
            KeyCode::Up => self.move_scrollbar_up(),
            KeyCode::Down => self.move_scrollbar_down(),
            _ => {
                self.main_text_area.input(key_event);
            }
        };
        Ok(())
    }

    fn move_scrollbar_up(&mut self) {
        if self.main_scroll_offset > 0 {
            self.main_scroll_offset -= 3;
        }
    }

    fn move_scrollbar_down(&mut self) {
        if self.main_scroll_offset < MESSAGES_SCROLL_RESERVE {
            self.main_scroll_offset += 3;
        }
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
