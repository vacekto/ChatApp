pub const FILES_DIR: &str = "./files/";
pub const FILES_FOR_TRANSFER: [&str; 3] = ["txt", "png", "jpg"];
pub const FILES_IMG_TO_ASCII: [&str; 2] = ["png", "jpg"];

pub const TCP_CHUNK_BUFFER_SIZE: usize = 8192;
// value 4 is compatible with server Tokio framing, should not change!!
pub const TCP_FRAME_SIZE_HEADER: usize = 4;
pub const MESSAGES_SCROLL_RESERVE: usize = 50;

pub const THEME_GRAY_GREEN_DARK: (u8, u8, u8) = (43, 51, 57);
pub const THEME_GRAY_GREEN_LIGHT: (u8, u8, u8) = (50, 61, 67);
pub const THEME_GREEN: (u8, u8, u8) = (131, 192, 146);
pub const THEME_YELLOW_DARK: (u8, u8, u8) = (219, 188, 127);
pub const THEME_YELLOW_LIGHT: (u8, u8, u8) = (92, 107, 85);
