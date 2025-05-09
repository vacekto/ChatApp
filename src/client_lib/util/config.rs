pub const FILES_DIR: &str = "files/";
pub const FILES_FOR_TRANSFER: [&str; 1] = ["txt"];

// value 4 is compatible with server framing, should not change!!
pub const TCP_FRAME_SIZE_HEADER: usize = 4;
pub const TCP_CHUNK_BUFFER_SIZE: usize = 8192;

pub const THEME_BG_DARK: (u8, u8, u8) = (43, 51, 57);
pub const THEME_BG_LIGHT: (u8, u8, u8) = (50, 61, 67);

pub const THEME_BORDER: (u8, u8, u8) = (131, 192, 146);
pub const THEME_SELECT: (u8, u8, u8) = (219, 188, 127);
pub const THEME_SELECT_BG: (u8, u8, u8) = (92, 107, 85);
