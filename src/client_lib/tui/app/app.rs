use crate::{
    client_lib::{
        global_states::{app_state::get_global_state, thread_logger::get_thread_runner},
        util::{
            config::THEME_GRAY_GREEN_LIGHT,
            types::{
                ActiveChannel, ActiveEntryInput, ActiveEntryScreen, ActiveScreen, ActiveStream,
                ChannelKind, FileSelector, Focus, ImgRender, MpscChannel, Notification, TuiUpdate,
            },
        },
    },
    shared_lib::{
        config::PUBLIC_ROOM_ID,
        types::{
            Channel, Chunk, ClientServerTuiMsg, DirectChannel, RegisterResponse, RoomUpdateTransit,
            TextMsg, TuiMsg, TuiRoom, User, UserClientData,
        },
    },
};
use anyhow::{bail, Result};
use ratatui::{
    crossterm::event::{self, Event},
    style::{Color, Style},
    widgets::Paragraph,
    DefaultTerminal, Frame,
};
use std::{
    collections::{HashMap, VecDeque},
    str::FromStr,
};
use tui_textarea::TextArea;
use uuid::Uuid;

pub struct App {
    pub username: String,
    pub id: Uuid,
    pub exit: bool,
    pub username_ta_login: TextArea<'static>,
    pub password_ta_login: TextArea<'static>,
    pub username_ta_register: TextArea<'static>,
    pub password_ta_register: TextArea<'static>,
    pub repeat_password_ta: TextArea<'static>,
    pub main_text_area: TextArea<'static>,
    pub room_channels: Vec<TuiRoom>,
    pub direct_channels: Vec<DirectChannel>,
    pub data_streams: HashMap<Uuid, ActiveStream>,
    pub active_channel: ActiveChannel,
    pub active_screen: ActiveScreen,
    pub active_entry_input: ActiveEntryInput,
    pub active_entry_screen: ActiveEntryScreen,
    pub display_file_selector: bool,
    pub file_selector: FileSelector,
    pub login_screen_notification: Option<Notification>,
    pub main_scroll_offset: usize,
    pub tui_channel: MpscChannel<TuiUpdate, TuiUpdate>,
    pub tx_tui_tcp_msg: crossbeam::channel::Sender<ClientServerTuiMsg>,
    pub tx_tui_tcp_file: crossbeam::channel::Sender<Chunk>,
    pub focus: Focus,
}

impl App {
    pub fn new() -> Self {
        let mut state = get_global_state();

        let tx_tui_tcp_msg = state.tui_tcp_msg_channel.tx.clone();
        let tx_tui_tcp_file = state.tui_tcp_file_channel.tx.clone();
        let tx_tui_update = state.tui_update_channel.tx.clone();
        let rx_tui_update = state
            .tui_update_channel
            .rx
            .take()
            .expect("rx_tui_update is already taken");

        drop(state);

        App {
            username: String::new(),
            id: Uuid::nil(),
            exit: false,
            username_ta_login: TextArea::default(),
            password_ta_login: TextArea::default(),
            password_ta_register: TextArea::default(),
            username_ta_register: TextArea::default(),
            repeat_password_ta: TextArea::default(),
            main_text_area: TextArea::default(),
            active_channel: ActiveChannel {
                id: None,
                kind: ChannelKind::Room,
            },
            direct_channels: vec![],
            room_channels: vec![],
            data_streams: HashMap::new(),
            active_screen: ActiveScreen::Entry,
            active_entry_screen: ActiveEntryScreen::Login,
            active_entry_input: ActiveEntryInput::Username,
            display_file_selector: false,
            file_selector: FileSelector::new(),
            login_screen_notification: None,
            main_scroll_offset: 0,
            tui_channel: MpscChannel {
                tx: tx_tui_update,
                rx: Some(rx_tui_update),
            },
            tx_tui_tcp_msg,
            tx_tui_tcp_file,
            focus: Focus::Messages,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.listen_for_tui_events();

        let rx_tui = self.tui_channel.rx.take().unwrap();
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;

            match rx_tui.recv()? {
                TuiUpdate::CrosstermEvent(e) => self.handle_events(e)?,
                TuiUpdate::Img(img) => self.handle_img_render(img)?,
                TuiUpdate::Auth(data) => self.handle_auth_response(data),
                TuiUpdate::RegisterResponse(res) => self.handle_register_response(res),
                TuiUpdate::UserJoinedRoom(update) => self.handle_user_joined_room(update),
                TuiUpdate::UserLeftRoom(update) => self.handle_user_left_room(update),
                TuiUpdate::Text(msg) => self.handle_text_message(msg),
                TuiUpdate::User(data) => self.handle_init_data(data),
                TuiUpdate::UserConnected(user) => self.handle_user_connected(user),
                TuiUpdate::UserDisconnected(user) => self.handle_user_disconnected(user),
            }
        }

        Ok(())
    }

    fn handle_register_response(&mut self, res: RegisterResponse) {
        match res {
            RegisterResponse::Failure(msg) => {
                self.login_screen_notification = Some(Notification::Failure(msg));
            }
            RegisterResponse::Success(user) => {
                let msg = format!("Account with username {} was created.", user.username);
                self.login_screen_notification = Some(Notification::Success(msg));

                self.password_ta_register = TextArea::default();
                self.username_ta_register = TextArea::default();
                self.repeat_password_ta = TextArea::default();
                self.active_entry_screen = ActiveEntryScreen::Login;
                self.active_entry_input = ActiveEntryInput::Username;
            }
        }
    }

    fn handle_user_connected(&mut self, user: User) {
        for room in &mut self.room_channels {
            if room.users.contains(&user) && !room.users_online.contains(&user) {
                if room.id == Uuid::from_str(PUBLIC_ROOM_ID).unwrap() {
                    let dr = DirectChannel {
                        messages: VecDeque::new(),
                        user: user.clone(),
                    };
                    self.direct_channels.push(dr);
                }
                room.users_online.push(user.clone());
            }
        }
    }

    fn handle_user_disconnected(&mut self, user: User) {
        if let Some(id) = self.active_channel.id {
            if user.id == id {
                self.active_channel.id = None;
            }
        }

        for room in &mut self.room_channels {
            if room.users.contains(&user) {
                if room.id == Uuid::from_str(PUBLIC_ROOM_ID).unwrap() {
                    self.direct_channels.retain(|dr| dr.user.id != user.id);
                }
                room.users_online.retain(|u| u.id != user.id);
            }
        }
    }

    fn handle_user_left_room(&mut self, update: RoomUpdateTransit) {
        if let Some(room) = self
            .room_channels
            .iter_mut()
            .find(|r| r.id == update.room_id)
        {
            room.users.retain_mut(|u| u.id != update.user.id);

            if room.id == Uuid::from_str(PUBLIC_ROOM_ID).unwrap() {
                self.direct_channels.retain(|r| r.user.id != update.user.id);
            }
        };
    }

    fn handle_user_joined_room(&mut self, update: RoomUpdateTransit) {
        if let Some(room) = self
            .room_channels
            .iter_mut()
            .find(|r| r.id == update.room_id)
        {
            room.users.push(update.user.clone());

            // if update.room_id == Uuid::from_str(PUBLIC_ROOM_ID).unwrap() {
            //     let new_channel = DirectChannel {
            //         messages: VecDeque::new(),
            //         user: update.user,
            //     };

            //     self.direct_channels.push(new_channel);
            // };
        };
    }

    fn handle_init_data(&mut self, data: UserClientData) {
        for mut room in data.rooms {
            room.users.retain(|u| u.username != self.username);

            if room.id == Uuid::from_str(PUBLIC_ROOM_ID).unwrap() {
                for user in &room.users {
                    let dir = DirectChannel {
                        messages: VecDeque::new(),
                        user: user.clone(),
                    };
                    self.direct_channels.push(dir);
                }
            }

            self.room_channels.push(room);
        }
    }

    pub fn switch_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Contacts => Focus::Messages,
            Focus::Messages => Focus::Contacts,
        };
    }

    fn listen_for_tui_events(&self) {
        let th_runner = get_thread_runner();
        let tx = self.tui_channel.tx.clone();

        th_runner.spawn("events listener", true, move || loop {
            let e = event::read()?;
            tx.send(TuiUpdate::CrosstermEvent(e))?;
        });
    }

    fn handle_img_render(&mut self, img: ImgRender) -> Result<()> {
        let messages = match img.from {
            Channel::Room(id) => self.get_room_messages(id),
            Channel::User(id) => self.get_direct_messages(id),
        };

        match messages {
            None => bail!("no messages found fo {:?}", img),
            Some(m) => m.push_front(TuiMsg::Img(img)),
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let background = Paragraph::new("").style(Style::default().bg(Color::Rgb(
            THEME_GRAY_GREEN_LIGHT.0,
            THEME_GRAY_GREEN_LIGHT.1,
            THEME_GRAY_GREEN_LIGHT.2,
        )));
        frame.render_widget(background, frame.area());
        frame.render_widget(&mut *self, frame.area());

        if self.display_file_selector {
            frame.render_widget(&mut self.file_selector, frame.area());
        }
    }

    pub fn init(&mut self, init: User) {
        self.username = init.username;
        self.id = init.id;
    }

    pub fn logout(&mut self) -> Result<()> {
        let msg = ClientServerTuiMsg::Logout;
        self.tx_tui_tcp_msg.send(msg)?;
        self.active_screen = ActiveScreen::Entry;
        self.active_entry_screen = ActiveEntryScreen::Login;
        self.direct_channels = vec![];
        self.room_channels = vec![];
        self.main_text_area = TextArea::default();

        Ok(())
    }

    fn handle_events(&mut self, event: Event) -> Result<()> {
        match (&self.active_screen, self.display_file_selector) {
            (ActiveScreen::Entry, _) => self.handle_entry_key_event(event)?,
            (ActiveScreen::Main, true) => self.handle_file_selector_key_event(event)?,
            (ActiveScreen::Main, false) => self.handle_main_key_event(event)?,
        }

        Ok(())
    }

    pub fn send_message(&mut self) -> Result<()> {
        let id = match self.active_channel.id {
            None => return Ok(()),
            Some(id) => id,
        };

        let state = get_global_state();
        let tx_tui_tcp = state.tui_tcp_msg_channel.tx.clone();
        drop(state);

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
            messages.push_front(TuiMsg::TextMsg(msg.clone()));
        };

        let msg = ClientServerTuiMsg::Text(msg);

        tx_tui_tcp.send(msg)?;
        self.main_text_area = TextArea::default();

        Ok(())
    }

    pub fn exit(&mut self) {
        self.exit = true;
    }
}
