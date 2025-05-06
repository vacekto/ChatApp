use crate::{
    client_lib::{
        global_states::app_state::get_global_state,
        util::types::{ActiveChannel, ActiveScreen, ActiveStream, ChannelKind, TuiUpdate},
    },
    shared_lib::types::{DirectChannel, InitClientData, RoomChannel, TuiServerMsg},
};
use anyhow::Result;
use ratatui::{
    crossterm::event::{Event, KeyEventKind},
    DefaultTerminal, Frame,
};
use std::{
    collections::HashMap,
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
    pub _active_streams: HashMap<Uuid, ActiveStream>,
    pub active_screen: ActiveScreen,
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
            _active_streams: HashMap::new(),
            active_screen: ActiveScreen::Login,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let (tx_tui, rx_tui) = mpsc::channel::<TuiUpdate>();

        self.listen_for_server(tx_tui.clone())?;
        self.listen_for_events(tx_tui.clone())?;

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            match rx_tui.recv()? {
                TuiUpdate::Event(e) => self.handle_events(e)?,
                TuiUpdate::ServerMsg(msg) => self.handle_server_msg(msg)?,
            }
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
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
        match self.active_screen {
            ActiveScreen::Login => {
                match e {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        self.handle_login_key_event(key_event)?
                    }

                    _ => {}
                };
            }
            ActiveScreen::Main => {
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
}
