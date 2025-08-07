use anyhow::{Result, bail};
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event},
    style::{Color, Style},
    widgets::Paragraph,
};
use shared::{
    config::PUBLIC_ROOM_ID,
    types::{
        Channel, ChannelMsg, Chunk, ClientServerAuthMsg, ClientServerMsg, DirectChannel, ImgRender,
        JoinRoomNotification, LeaveRoomNotification, RegisterResponse, RoomData, TextMsg, TuiRoom,
        User, UserInitData,
    },
};
use std::{
    collections::{HashMap, VecDeque},
    str::FromStr,
};
use tokio::select;
use tui_textarea::TextArea;
use uuid::Uuid;

use crate::{
    tui::accessories::{
        create_room::create_room::RoomCreator, file_selector::file_selector::FileSelector,
    },
    util::{
        config::THEME_GRAY_GREEN_LIGHT,
        types::{
            ActiveChannel, ActiveCreateRoomInput, ActiveEntryInput, ActiveEntryScreen,
            ActiveScreen, ActiveStream, ChannelKind, Focus, Notification, TuiUpdate,
        },
    },
};

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
    pub active_create_room_input: ActiveCreateRoomInput,
    pub display_file_selector: bool,
    pub display_room_creator: bool,
    pub file_selector: FileSelector,
    pub room_creator: RoomCreator,
    pub login_screen_notification: Option<Notification>,
    pub main_scroll_offset: usize,
    pub tx_tui_ws_msg: tokio::sync::mpsc::Sender<ClientServerMsg>,
    pub tx_tui_ws_file: tokio::sync::mpsc::Sender<Chunk>,
    pub focus: Focus,
    pub rx_ws_tui: tokio::sync::mpsc::Receiver<TuiUpdate>,
    pub tx_events_tui: tokio::sync::mpsc::Sender<Event>,
    pub rx_events_tui: tokio::sync::mpsc::Receiver<Event>,
    pub tx_tui_ws_auth: tokio::sync::mpsc::Sender<ClientServerAuthMsg>,
}

impl App {
    pub fn new(
        rx_ws_tui: tokio::sync::mpsc::Receiver<TuiUpdate>,
        tx_tui_ws_file: tokio::sync::mpsc::Sender<Chunk>,
        tx_tui_ws_msg: tokio::sync::mpsc::Sender<ClientServerMsg>,
        tx_tui_ws_auth: tokio::sync::mpsc::Sender<ClientServerAuthMsg>,
    ) -> Self {
        let (tx_events_tui, rx_events_tui) = tokio::sync::mpsc::channel(20);
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
            active_entry_screen: ActiveEntryScreen::ASLogin,
            active_entry_input: ActiveEntryInput::Username,
            active_create_room_input: ActiveCreateRoomInput::Name,
            display_file_selector: false,
            display_room_creator: false,
            file_selector: FileSelector::new(),
            room_creator: RoomCreator::new(),
            login_screen_notification: None,
            main_scroll_offset: 0,
            rx_ws_tui,
            tx_tui_ws_msg,
            tx_tui_ws_file,
            tx_tui_ws_auth: tx_tui_ws_auth,
            focus: Focus::Messages,
            rx_events_tui,
            tx_events_tui,
        }
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.listen_for_tui_events().await;

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;

            select! {
                result = self.rx_events_tui.recv() => if let Some(e) = result {
                    self.handle_events(e).await?
                },

                result = self.rx_ws_tui.recv() => if let Some(msg) = result {
                    match msg{
                        TuiUpdate::Img(img) => self.handle_img_render(img)?,
                        TuiUpdate::Auth(data) => self.handle_auth_response(data),
                        TuiUpdate::RegisterResponse(res) => self.handle_register_response(res),
                        TuiUpdate::UserJoinedRoom(update) => self.handle_user_joined_room(update),
                        TuiUpdate::UserLeftRoom(update) => self.handle_user_left_room(update),
                        TuiUpdate::Text(msg) => self.handle_text_message(msg),
                        TuiUpdate::Init(data) => self.handle_init_data(data),
                        TuiUpdate::UserConnected(user) => self.handle_user_connected(user),
                        TuiUpdate::UserDisconnected(user) => self.handle_user_disconnected(user),
                        TuiUpdate::JoinRoom(res) => self.handle_join_room(res),
                    }
                },

            }
        }

        Ok(())
    }

    fn handle_join_room(&mut self, res: Result<RoomData, String>) {
        match res {
            Err(msg) => self.room_creator.notification = Some(msg),
            Ok(room) => {
                let room = TuiRoom {
                    id: room.id,
                    name: room.name,
                    messages: VecDeque::new(),
                    users: room.users,
                    users_online: room.users_online,
                };
                self.active_channel = ActiveChannel {
                    id: Some(room.id),
                    kind: ChannelKind::Room,
                };
                self.room_channels.push(room);
                self.display_room_creator = false;
            }
        }
    }

    fn handle_register_response(&mut self, res: RegisterResponse) {
        match res {
            RegisterResponse::Err(msg) => {
                self.login_screen_notification = Some(Notification::Failure(msg));
            }
            RegisterResponse::Ok(user) => {
                let msg = format!("Account with username {} was created.", user.username);
                self.login_screen_notification = Some(Notification::Success(msg));

                self.password_ta_register = TextArea::default();
                self.username_ta_register = TextArea::default();
                self.repeat_password_ta = TextArea::default();
                self.active_entry_screen = ActiveEntryScreen::ASLogin;
                self.active_entry_input = ActiveEntryInput::Username;
            }
        }
    }

    fn handle_user_connected(&mut self, user: User) {
        for room in &mut self.room_channels {
            if room.users.contains(&user) && !room.users_online.contains(&user) {
                if room.id == Uuid::from_str(PUBLIC_ROOM_ID).unwrap() {
                    if user.username == self.username {
                        continue;
                    }
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

    fn handle_user_left_room(&mut self, update: LeaveRoomNotification) {
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

    fn handle_user_joined_room(&mut self, update: JoinRoomNotification) {
        if let Some(room) = self
            .room_channels
            .iter_mut()
            .find(|r| r.id == update.room_id)
        {
            room.users.push(update.user.clone());
        };
    }

    fn handle_init_data(&mut self, data: UserInitData) {
        for room in data.rooms {
            if room.id == Uuid::from_str(PUBLIC_ROOM_ID).unwrap() {
                for user in &room.users_online {
                    if user.username == self.username {
                        continue;
                    }
                    let dir = DirectChannel {
                        messages: VecDeque::new(),
                        user: user.clone(),
                    };
                    self.direct_channels.push(dir);
                }
            }

            let room = TuiRoom {
                id: room.id,
                name: room.name,
                messages: VecDeque::new(),
                users: room.users,
                users_online: room.users_online,
            };

            self.room_channels.push(room);
        }
    }

    pub fn switch_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Contacts => Focus::Messages,
            Focus::Messages => Focus::Contacts,
        };
    }

    async fn listen_for_tui_events(&self) {
        let tx = self.tx_events_tui.clone();

        tokio::spawn(async move {
            loop {
                let handle = tokio::task::spawn_blocking(|| event::read().unwrap());
                let event = handle.await.unwrap();
                tx.send(event).await.unwrap();
            }
        });
    }

    fn handle_img_render(&mut self, img: ImgRender) -> Result<()> {
        let messages = match img.to {
            Channel::Room(id) => self.get_room_messages(id),
            Channel::User(id) => self.get_direct_messages(id),
        };

        match messages {
            None => bail!("no messages found fo {:?}", img),
            Some(m) => m.push_front(ChannelMsg::Img(img)),
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

        if self.display_room_creator {
            frame.render_widget(&mut self.room_creator, frame.area());
        }
    }

    pub fn init(&mut self, init: User) {
        self.username = init.username;
        self.id = init.id;
    }

    pub async fn logout(&mut self) -> Result<()> {
        let msg = ClientServerMsg::Logout;
        self.tx_tui_ws_msg.send(msg).await?;
        self.active_screen = ActiveScreen::Entry;
        self.active_entry_screen = ActiveEntryScreen::ASLogin;
        self.direct_channels = vec![];
        self.room_channels = vec![];
        self.main_text_area = TextArea::default();
        self.login_screen_notification = None;

        Ok(())
    }

    async fn handle_events(&mut self, event: Event) -> Result<()> {
        match (
            &self.active_screen,
            self.display_file_selector,
            self.display_room_creator,
        ) {
            (ActiveScreen::Entry, _, _) => self.handle_entry_screen_event(event).await?,
            (ActiveScreen::Main, false, false) => self.handle_main_screen_event(event).await?,
            (ActiveScreen::Main, true, _) => self.handle_file_selector_key_event(event).await?,
            (ActiveScreen::Main, _, true) => self.handle_create_room_event(event).await?,
        }

        Ok(())
    }

    pub async fn send_message(&mut self) -> Result<()> {
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
            messages.push_front(ChannelMsg::TextMsg(msg.clone()));
        };

        let msg = ClientServerMsg::Text(msg);

        self.tx_tui_ws_msg.send(msg).await?;
        self.main_text_area = TextArea::default();

        Ok(())
    }

    pub fn exit(&mut self) {
        self.exit = true;
    }
}
