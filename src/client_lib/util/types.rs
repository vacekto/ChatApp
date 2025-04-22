use std::{fs::File, io::Write};

use anyhow::Result;

#[derive(Debug)]
pub struct ActiveStream {
    pub file_handle: File,
    pub written: u64,
    pub size: u64,
}

impl ActiveStream {
    pub async fn write_all(&mut self, c: &[u8]) -> Result<()> {
        Ok(self.file_handle.write_all(c)?)
    }
}
