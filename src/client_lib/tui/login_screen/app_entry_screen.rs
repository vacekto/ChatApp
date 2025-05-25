use std::io::Write;

use crate::{
    client_lib::{
        global_states::app_state::get_global_state,
        tui::app::app::App,
        util::types::{ActiveEntryInput, ActiveEntryScreen, Notification},
        write_server::frame_data,
    },
    shared_lib::{
        config::USERNAME_RE_PATTERN,
        types::{AuthData, ClientServerConnectMsg, RegisterData},
    },
};
use anyhow::Result;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use regex::Regex;

impl App {
    fn switch_entry_screen(&mut self) {
        self.active_entry_screen = match self.active_entry_screen {
            ActiveEntryScreen::Login => ActiveEntryScreen::Register,
            ActiveEntryScreen::Register => {
                if self.active_entry_input == ActiveEntryInput::RepeatPassword {
                    self.active_entry_input = ActiveEntryInput::Username;
                }
                ActiveEntryScreen::Login
            }
        };
    }

    pub fn handle_entry_key_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.exit()
                    }
                    KeyCode::Tab => self.switch_entry_screen(),
                    KeyCode::Esc => self.exit(),
                    KeyCode::Enter => self.handle_entry_enter()?,
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
        match self.active_entry_screen {
            ActiveEntryScreen::Login => {
                match self.active_entry_input {
                    ActiveEntryInput::Username => {
                        self.username_ta_login.input(key_event);
                    }
                    ActiveEntryInput::Password => {
                        self.password_ta_login.input(key_event);
                    }
                    ActiveEntryInput::RepeatPassword => unreachable!(),
                };
            }
            ActiveEntryScreen::Register => match self.active_entry_input {
                ActiveEntryInput::Username => {
                    self.username_ta_register.input(key_event);
                }
                ActiveEntryInput::Password => {
                    self.password_ta_register.input(key_event);
                }
                ActiveEntryInput::RepeatPassword => {
                    self.repeat_password_ta.input(key_event);
                }
            },
        }
    }

    fn move_active_input_up(&mut self) {
        match self.active_entry_screen {
            ActiveEntryScreen::Login => {
                self.active_entry_input = match self.active_entry_input {
                    ActiveEntryInput::Username => ActiveEntryInput::Username,
                    ActiveEntryInput::Password => ActiveEntryInput::Username,
                    _ => unreachable!(),
                }
            }
            ActiveEntryScreen::Register => {
                self.active_entry_input = match self.active_entry_input {
                    ActiveEntryInput::Username => ActiveEntryInput::Username,
                    ActiveEntryInput::Password => ActiveEntryInput::Username,
                    ActiveEntryInput::RepeatPassword => ActiveEntryInput::Password,
                }
            }
        }
    }

    fn move_active_input_down(&mut self) {
        match self.active_entry_screen {
            ActiveEntryScreen::Login => {
                self.active_entry_input = match self.active_entry_input {
                    ActiveEntryInput::Username => ActiveEntryInput::Password,
                    ActiveEntryInput::Password => ActiveEntryInput::Password,
                    _ => unreachable!(),
                }
            }
            ActiveEntryScreen::Register => {
                self.active_entry_input = match self.active_entry_input {
                    ActiveEntryInput::Username => ActiveEntryInput::Password,
                    ActiveEntryInput::Password => ActiveEntryInput::RepeatPassword,
                    ActiveEntryInput::RepeatPassword => ActiveEntryInput::RepeatPassword,
                }
            }
        }
    }

    fn handle_entry_enter(&mut self) -> Result<()> {
        match self.active_entry_screen {
            ActiveEntryScreen::Login => self.handle_auth()?,
            ActiveEntryScreen::Register => self.handle_register()?,
        };

        Ok(())
    }

    fn handle_auth(&mut self) -> Result<()> {
        let username = String::from(self.username_ta_login.lines().join("").trim());
        let password = String::from(self.username_ta_login.lines().join("").trim());

        if let Err(msg) = self.validate_login() {
            self.login_screen_notification = Some(Notification::Failure(msg));
            return Ok(());
        };

        let mut state = get_global_state();
        let data = AuthData { username, password };

        let msg = ClientServerConnectMsg::Login(data);
        let serialized = bincode::serialize(&msg)?;
        let framed = frame_data(&serialized);
        state.tcp.write_all(&framed)?;

        Ok(())
    }

    fn handle_register(&mut self) -> Result<()> {
        let username = String::from(self.username_ta_register.lines().join("\n").trim());
        let password = String::from(self.username_ta_register.lines().join("\n").trim());

        if let Err(msg) = self.validate_register() {
            self.login_screen_notification = Some(Notification::Failure(msg));
            return Ok(());
        };

        let mut state = get_global_state();

        let data = RegisterData { username, password };
        let msg = ClientServerConnectMsg::Register(data);
        let serialized = bincode::serialize(&msg)?;
        let framed = frame_data(&serialized);
        state.tcp.write_all(&framed)?;

        Ok(())
    }

    fn validate_login(&mut self) -> Result<(), String> {
        let username_pattern = USERNAME_RE_PATTERN;
        let username_re = Regex::new(username_pattern).unwrap();

        let password_allowed_pattern = r"^[A-Za-z\d!@#$%^&*()_+]{8,32}$";
        let password_allowed_re = Regex::new(password_allowed_pattern).unwrap();

        let username = String::from(self.username_ta_login.lines().join("").trim());
        let password = String::from(self.password_ta_login.lines().join("").trim());

        let username_error_msg  = String::from("Username must start with a letter, not contain special characters ouside of \"_\" and have length between 7 to 29");
        let password_error_msg =
            String::from("Password must contain at least one lowercase and uppercase letter, digit and have length between 8 to 32");

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

        Ok(())
    }

    fn validate_register(&self) -> Result<(), String> {
        let username_pattern = USERNAME_RE_PATTERN;
        let username_re = Regex::new(username_pattern).unwrap();

        let password_allowed_pattern = r"^[A-Za-z\d!@#$%^&*()_+]{8,32}$";
        let password_allowed_re = Regex::new(password_allowed_pattern).unwrap();

        let username = String::from(self.username_ta_register.lines().join("").trim());
        let password = String::from(self.password_ta_register.lines().join("").trim());
        let repeat_password = String::from(self.repeat_password_ta.lines().join("").trim());

        let username_error_msg  = String::from("Username must start with a letter, not contain special characters ouside of \"_\" and have length between 7 to 29");
        let password_error_msg =
            String::from("Password must contain at least one lowercase and uppercase letter, digit and have length between 8 to 32");
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
