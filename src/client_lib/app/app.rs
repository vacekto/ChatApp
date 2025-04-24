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
        util::types::{ActiveStream, TuiUpdate},
    },
    shared_lib::{
        config::PUBLIC_ROOM_ID_STR,
        types::{
            Channel, ChannelMsg, ClientServerMsg, DirectChannel, InitClientData, RoomChannel,
            RoomJoinNotification, ServerClientMsg, TextMsg, User,
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
    pub active_channel: Channel,
    pub _active_streams: HashMap<Uuid, ActiveStream>,
}

impl App {
    pub fn new(tx_tui_write: mpsc::Sender<ClientServerMsg>, username: String) -> Self {
        App {
            username,
            id: Uuid::nil(),
            exit: false,
            text_area: TextArea::default(),
            tx_tui_write,
            active_channel: Channel::Room(Uuid::from_str(PUBLIC_ROOM_ID_STR).unwrap()),
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
            ServerClientMsg::InitClient(init) => self.init_app_state(init),
            ServerClientMsg::Text(msg) => self.handle_text_message(msg),
            ServerClientMsg::UserJoinedRoom(join) => self.handle_join_notification(join),
            ServerClientMsg::JoinInv(room) => self.handle_room_invitation(room),
        };

        Ok(())
    }

    fn init_app_state(&mut self, init: InitClientData) {
        self.id = init.id;
        self.room_channels = init.room_channels;
    }

    fn handle_room_invitation(&mut self, room: RoomChannel) {
        self.room_channels.push(room);
    }

    fn handle_join_notification(&mut self, join: RoomJoinNotification) {
        let messages = self.get_messages(&Channel::Room(join.room_id));

        if let Some(m) = messages {
            m.push(ChannelMsg::JoinNotification(join));
        }
    }

    pub fn handle_text_message(&mut self, msg: TextMsg) {
        let messages = self.get_messages(&msg.to);
        if let Some(m) = messages {
            m.push(ChannelMsg::TextMsg(msg));
            return;
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    pub fn get_messages(&mut self, channel: &Channel) -> Option<&mut Vec<ChannelMsg>> {
        match channel {
            Channel::User(id) => {
                let res = self.direct_channels.iter_mut().find(|c| &c.user.id == id);
                match res {
                    Some(c) => Some(&mut c.messages),
                    None => None,
                }
            }
            Channel::Room(id) => {
                let res = self.room_channels.iter_mut().find(|c| &c.id == id);
                match res {
                    Some(c) => Some(&mut c.messages),
                    None => None,
                }
            }
        }
    }

    fn handle_events(&mut self, e: Event) -> Result<()> {
        // if let Event::Key(key_event) = e {
        //     if key_event.code == KeyCode::Char('q') {
        //         self.exit()
        //     } else {
        //         self.text_area.input(e);
        //     }
        // }

        //  && key_event.code == KeyCode::Char('q') {}

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
            _ => {
                self.text_area.input(key_event);
            }
        };

        Ok(())
    }

    fn send_message(&mut self) -> Result<()> {
        let text = self.text_area.lines().join("\n");

        let msg = ClientServerMsg::Text(TextMsg {
            text,
            from: User {
                username: self.username.clone(),
                id: self.id,
            },
            to: Channel::Room(Uuid::from_str(PUBLIC_ROOM_ID_STR)?),
        });

        self.tx_tui_write.send(msg)?;
        self.text_area = TextArea::default();

        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}
