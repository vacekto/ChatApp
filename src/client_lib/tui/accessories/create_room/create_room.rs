use crate::client_lib::util::types::{ActiveCreateRoomInput, RoomAction};
use tui_textarea::TextArea;

pub struct RoomCreator {
    pub room_name_ta: TextArea<'static>,
    pub room_password_ta: TextArea<'static>,
    pub notification: Option<String>,
    pub active_input: ActiveCreateRoomInput,
    pub active_action: RoomAction,
}

impl RoomCreator {
    pub fn new() -> Self {
        Self {
            room_name_ta: TextArea::default(),
            room_password_ta: TextArea::default(),
            notification: None,
            active_input: ActiveCreateRoomInput::Name,
            active_action: RoomAction::Create,
        }
    }

    pub fn move_active_input_up(&mut self) {
        self.active_input = ActiveCreateRoomInput::Name;
    }

    pub fn move_active_input_down(&mut self) {
        self.active_input = ActiveCreateRoomInput::Password;
    }

    pub fn switch_action(&mut self) {
        self.active_action = match self.active_action {
            RoomAction::Create => RoomAction::Join,
            RoomAction::Join => RoomAction::Create,
        }
    }
}
