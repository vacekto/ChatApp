use once_cell::sync::OnceCell;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    process::{Child, Command, Stdio},
    sync::Mutex,
};

pub const CONSOLE_LOG_FILE_PATH: &str = "log.txt";
static GLOBAL: OnceCell<Mutex<ConsoleLogger>> = OnceCell::new();

pub fn log_to_console(msg: &str) {
    #[cfg(debug_assertions)]
    log(msg)
}

fn log(msg: &str) {
    let mut logger = match GLOBAL.get() {
        Some(l) => l.lock().unwrap(),

        None => {
            GLOBAL.set(Mutex::new(ConsoleLogger::new())).unwrap();
            GLOBAL.get().unwrap().lock().unwrap()
        }
    };
    logger.log(msg);
}

#[cfg(debug_assertions)]
pub fn close_console_logger() {
    let mut logger = GLOBAL.get().unwrap().lock().unwrap();
    logger.close_terminal();
}

#[derive(Debug)]
struct ConsoleLogger {
    file: File,
    child: Child,
}

impl ConsoleLogger {
    fn close_terminal(&mut self) {
        self.log("closing");
        if let Err(e) = self.child.kill() {
            eprintln!("Failed to kill terminal: {e}");
        }
        let _ = self.child.wait();
    }
    pub fn new() -> Self {
        let path = CONSOLE_LOG_FILE_PATH;

        let path = Path::new(&path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }

        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .unwrap();

        let cmd = String::from(format!("tail -f {}", CONSOLE_LOG_FILE_PATH));
        let child = Command::new("kitty")
            .args(["--", "bash", "-c", &cmd])
            .stderr(Stdio::null()) // <--- suppress error output
            .spawn()
            .expect("Failed to open terminal");

        Self { file, child }
    }

    pub fn log(&mut self, msg: &str) {
        self.file.write_all(msg.as_bytes()).unwrap();
        self.file.write_all(b"\n\n").unwrap();
    }
}
