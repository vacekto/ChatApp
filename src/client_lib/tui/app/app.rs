use crate::{
    client_lib::{
        global_states::{app_state::get_global_state, thread_logger::get_thread_runner},
        util::{
            config::THEME_GRAY_GREEN_LIGHT,
            types::{
                ActiveChannel, ActiveScreen, ActiveStream, ChannelKind, FileSelector, Focus,
                ImgRender, MpscChannel, TuiUpdate,
            },
        },
    },
    shared_lib::{
        config::PUBLIC_ROOM_ID,
        types::{
            Channel, Chunk, ClientRoomUpdateTransit, ClientServerMsg, DirectChannel,
            InitPersistedUserData, InitUserData, RoomChannel, TextMsg, TuiMsg, User,
        },
    },
};
use anyhow::{bail, Result};
use ratatui::{
    crossterm::event::{self, Event, KeyEventKind},
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
    pub login_text_area: TextArea<'static>,
    pub main_text_area: TextArea<'static>,
    pub room_channels: Vec<RoomChannel>,
    pub direct_channels: Vec<DirectChannel>,
    pub active_channel: ActiveChannel,
    pub data_streams: HashMap<Uuid, ActiveStream>,
    pub active_screen: ActiveScreen,
    pub display_file_selector: bool,
    pub file_selector: FileSelector,
    pub login_notification: Option<String>,
    pub main_scroll_offset: usize,
    pub tui_channel: MpscChannel<TuiUpdate, TuiUpdate>,
    pub tx_tui_tcp_msg: crossbeam::channel::Sender<ClientServerMsg>,
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
            login_text_area: TextArea::default(),
            main_text_area: TextArea::default(),
            active_channel: ActiveChannel {
                id: None,
                kind: ChannelKind::Room,
            },
            direct_channels: vec![],
            room_channels: vec![],
            data_streams: HashMap::new(),
            active_screen: ActiveScreen::Login,
            display_file_selector: false,
            file_selector: FileSelector::new(),
            login_notification: None,
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
        self.listen_for_events();

        let rx_tui = self.tui_channel.rx.take().unwrap();
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;

            match rx_tui.recv()? {
                TuiUpdate::CrosstermEvent(e) => self.handle_events(e)?,
                TuiUpdate::Img(img) => self.handle_img_render(img)?,
                TuiUpdate::Auth(data) => self.handle_auth_response(data),
                TuiUpdate::UserJoinedRoom(update) => self.handle_user_joined_room(update),
                TuiUpdate::UserLeftRoom(update) => self.handle_user_left_room(update),
                TuiUpdate::Text(msg) => self.handle_text_message(msg),
                TuiUpdate::UserInitData(data) => self.handle_init_data(data),
            }
        }

        Ok(())
    }

    fn handle_user_left_room(&mut self, update: ClientRoomUpdateTransit) {
        if let Some(room) = self
            .room_channels
            .iter_mut()
            .find(|r| r.id == update.room_id)
        {
            room.users.retain_mut(|u| u.id != update.user.id);

            if room.id == Uuid::from_str(PUBLIC_ROOM_ID).unwrap() {
                self.direct_channels.retain(|r| r.user.id != update.user.id);
                {}
            }
        };
    }

    fn handle_user_joined_room(&mut self, update: ClientRoomUpdateTransit) {
        if let Some(room) = self
            .room_channels
            .iter_mut()
            .find(|r| r.id == update.room_id)
        {
            room.users.push(update.user.clone());

            let new_channel = DirectChannel {
                messages: VecDeque::new(),
                user: update.user,
            };

            self.direct_channels.push(new_channel);
        };
    }

    fn handle_init_data(&mut self, data: InitPersistedUserData) {
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

    fn listen_for_events(&self) {
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

    pub fn init(&mut self, init: InitUserData) {
        self.username = init.username;
        self.id = init.id;
    }

    pub fn logout(&mut self) -> Result<()> {
        let state = get_global_state();
        let tx_tui_tcp = state.tui_tcp_msg_channel.tx.clone();
        drop(state);

        let msg = ClientServerMsg::Logout;
        tx_tui_tcp.send(msg)?;
        self.active_screen = ActiveScreen::Login;
        self.direct_channels = vec![];
        self.room_channels = vec![];

        Ok(())
    }

    fn handle_events(&mut self, e: Event) -> Result<()> {
        match (&self.active_screen, self.display_file_selector) {
            (ActiveScreen::Login, _) => {
                match e {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        self.handle_login_key_event(key_event)?
                    }
                    _ => {}
                };
            }
            (ActiveScreen::Main, true) => {
                match e {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        self.handle_file_selector_key_event(key_event)?
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

        let msg = ClientServerMsg::Text(msg);

        tx_tui_tcp.send(msg)?;
        self.main_text_area = TextArea::default();

        Ok(())
    }

    pub fn exit(&mut self) {
        self.exit = true;
    }
}
