use crate::{
    client_lib::{
        tui::app::App,
        util::{
            config::MESSAGES_SCROLL_RESERVE,
            types::{ActiveScreen, ChannelKind, Focus, Notification},
        },
    },
    shared_lib::types::{AuthResponse, Channel, TextMsg, TuiMsg},
};
use anyhow::Result;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::collections::VecDeque;
use tui_textarea::TextArea;
use uuid::Uuid;

impl App {
    pub fn handle_auth_response(&mut self, data: AuthResponse) {
        match data {
            AuthResponse::Failure(msg) => {
                self.login_screen_notification = Some(Notification::Failure(msg))
            }
            AuthResponse::Success(init) => {
                self.username = init.username;
                self.id = init.id;
                self.active_screen = ActiveScreen::Main;

                self.username_ta_login = TextArea::default();
                self.password_ta_login = TextArea::default();
                self.password_ta_register = TextArea::default();
                self.username_ta_register = TextArea::default();
                self.repeat_password_ta = TextArea::default();
            }
        }
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

    pub fn handle_main_screen_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match self.focus {
                    Focus::Contacts => self.handle_contacts_event(key_event)?,
                    Focus::Messages => self.handle_messages_event(key_event)?,
                };
            }
            _ => {}
        };

        Ok(())
    }

    fn handle_contacts_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.exit()
            }
            KeyCode::Tab => self.switch_focus(),
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
            KeyCode::Tab => self.switch_focus(),
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
