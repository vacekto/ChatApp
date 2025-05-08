pub const FILES_DIR: &str = "files/";
pub const FILES_FOR_TRANSFER: [&str; 1] = ["txt"];

// value 4 is compatible with server framing, should not change!!
pub const TCP_FRAME_SIZE_HEADER: usize = 4;
pub const TCP_CHUNK_BUFFER_SIZE: usize = 8192;
