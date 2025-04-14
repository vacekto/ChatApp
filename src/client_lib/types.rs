use tokio::{fs::File, io::AsyncWriteExt};

#[derive(Debug)]
pub struct ActiveStream {
    pub file_handle: File,
    pub written: u64,
}

impl ActiveStream {
    pub async fn write_all(&mut self, c: &[u8]) {
        self.file_handle.write_all(c).await.unwrap();
    }
}
