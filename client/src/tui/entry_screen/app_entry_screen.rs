use crate::{
    tui::app::app::App,
    util::types::{
        ActiveEntryInput::{Password, RepeatPassword, Username},
        ActiveEntryScreen::{ASLogin, ASRegister},
        Notification,
    },
};

use anyhow::Result;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use regex::Regex;
use shared::{
    config::{PASSWORD_ERROR_MSG, PASSWORD_RE_PATTERN, USERNAME_ERROR_MSG, USERNAME_RE_PATTERN},
    types::{AuthData, ClientServerAuthMsg, RegisterData},
};

impl App {
    fn switch_entry_screen(&mut self) {
        self.active_entry_screen = match self.active_entry_screen {
            ASLogin => ASRegister,
            ASRegister => {
                if self.active_entry_input == RepeatPassword {
                    self.active_entry_input = Username;
                }
                ASLogin
            }
        };
    }

    pub async fn handle_entry_screen_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.exit()
                    }
                    KeyCode::Tab => self.switch_entry_screen(),
                    KeyCode::Esc => self.exit(),
                    KeyCode::Enter => self.handle_entry_enter().await?,
                    KeyCode::Up => self.move_active_input_up(),
                    KeyCode::Down => self.move_active_input_down(),

                    _ => self.handle_input_event(key_event),
                };
            }
            _ => {}
        };

        Ok(())
    }

    fn handle_input_event(&mut self, key_event: KeyEvent) {
        match (&self.active_entry_screen, &self.active_entry_input) {
            (ASLogin, Username) => self.username_ta_login.input(key_event),
            (ASLogin, Password) => self.password_ta_login.input(key_event),
            (ASLogin, RepeatPassword) => unreachable!(),
            (ASRegister, Username) => self.username_ta_register.input(key_event),
            (ASRegister, Password) => self.password_ta_register.input(key_event),
            (ASRegister, RepeatPassword) => self.repeat_password_ta.input(key_event),
        };
    }

    fn move_active_input_up(&mut self) {
        self.active_entry_input = match (&self.active_entry_screen, &self.active_entry_input) {
            (ASLogin, Username) => Username,
            (ASLogin, Password) => Username,
            (ASLogin, RepeatPassword) => unreachable!(),
            (ASRegister, Username) => Username,
            (ASRegister, Password) => Username,
            (ASRegister, RepeatPassword) => Password,
        };
    }

    fn move_active_input_down(&mut self) {
        self.active_entry_input = match (&self.active_entry_screen, &self.active_entry_input) {
            (ASLogin, Username) => Password,
            (ASLogin, Password) => Password,
            (ASLogin, RepeatPassword) => unreachable!(),
            (ASRegister, Username) => Password,
            (ASRegister, Password) => RepeatPassword,
            (ASRegister, RepeatPassword) => RepeatPassword,
        };
    }

    async fn handle_entry_enter(&mut self) -> Result<()> {
        match self.active_entry_screen {
            ASLogin => self.handle_auth().await?,
            ASRegister => self.handle_register().await?,
        };

        Ok(())
    }

    async fn handle_auth(&mut self) -> Result<()> {
        let username = String::from(self.username_ta_login.lines().join("").trim());
        let pwd = String::from(self.password_ta_login.lines().join("").trim());

        if let Err(msg) = self.validate_login(&username, &pwd) {
            self.login_screen_notification = Some(Notification::Failure(msg));
            return Ok(());
        };

        // let mut state = get_global_state();

        let data = AuthData { username, pwd };

        let msg = ClientServerAuthMsg::Login(data);
        // const true_msg = ClientServerMsg::
        self.tx_tui_ws_auth.send(msg).await?;
        // let serialized = bincode::serialize(&msg)?;
        // let framed = frame_data(&serialized);
        // state.tcp.write_all(&framed)?;

        Ok(())
    }

    async fn handle_register(&mut self) -> Result<()> {
        let username = String::from(self.username_ta_register.lines().join("\n").trim());
        let password = String::from(self.password_ta_register.lines().join("\n").trim());
        let repeat_password = String::from(self.repeat_password_ta.lines().join("\n").trim());

        if let Err(msg) = self.validate_register(&username, &password, &repeat_password) {
            self.login_screen_notification = Some(Notification::Failure(msg));
            return Ok(());
        };

        // let mut state = get_global_state();

        let data = RegisterData {
            username,
            pwd: password,
        };
        let msg = ClientServerAuthMsg::Register(data);
        self.tx_tui_ws_auth.send(msg).await?;
        // let serialized = bincode::serialize(&msg)?;
        // let framed = frame_data(&serialized);
        // state.tcp.write_all(&framed)?;

        Ok(())
    }

    fn validate_login(&mut self, username: &str, password: &str) -> Result<(), String> {
        let username_pattern = USERNAME_RE_PATTERN;
        let username_re = Regex::new(username_pattern).unwrap();

        let password_allowed_pattern = PASSWORD_RE_PATTERN;
        let password_allowed_re = Regex::new(password_allowed_pattern).unwrap();

        let username_error_msg = String::from(USERNAME_ERROR_MSG);
        let password_error_msg = String::from(PASSWORD_ERROR_MSG);

        if !username_re.is_match(username) {
            return Err(username_error_msg);
        };

        if !password_allowed_re.is_match(password)
            || !password.chars().any(|c| c.is_lowercase())
            || !password.chars().any(|c| c.is_uppercase())
            || !password.chars().any(|c| c.is_ascii_digit())
        {
            return Err(password_error_msg);
        }

        Ok(())
    }

    fn validate_register(
        &self,
        username: &str,
        password: &str,
        repeat_password: &str,
    ) -> Result<(), String> {
        let username_pattern = USERNAME_RE_PATTERN;
        let username_re = Regex::new(username_pattern).unwrap();

        let password_allowed_pattern = r"^[A-Za-z\d!@#$%^&*()_+]{8,32}$";
        let password_allowed_re = Regex::new(password_allowed_pattern).unwrap();

        let username_error_msg = String::from(
            "Username must start with a letter, not contain special characters ouside of \"_\" and have length between 7 to 29",
        );
        let password_error_msg = String::from(
            "Password must contain at least one lowercase and uppercase letter, digit and have length between 8 to 32",
        );
        let repeat_password_error_msg =
            String::from("Password and Repeat password fields must match");

        if !username_re.is_match(&username) {
            return Err(username_error_msg);
        };

        if !password_allowed_re.is_match(&password)
            || !password.chars().any(|c| c.is_lowercase())
            || !password.chars().any(|c| c.is_uppercase())
            || !password.chars().any(|c| c.is_ascii_digit())
        {
            return Err(password_error_msg);
        }

        if repeat_password != password {
            return Err(repeat_password_error_msg);
        }
        Ok(())
    }
}
