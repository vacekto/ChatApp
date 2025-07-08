use anyhow::{self, Result};
use once_cell::sync::OnceCell;
use std::{
    sync::{Mutex, mpsc},
    thread,
};

static GLOBAL: OnceCell<Mutex<ThreadConstructor>> = OnceCell::new();

fn gen_constructor() -> std::sync::MutexGuard<'static, ThreadConstructor> {
    let mutex = match GLOBAL.get() {
        Some(v) => v,
        None => {
            GLOBAL.set(Mutex::new(ThreadConstructor::new())).unwrap();
            GLOBAL.get().unwrap()
        }
    };

    mutex.lock().expect("Global tcp stream instance poisoned!!")
}

pub fn get_thread_logger() -> ThreadLogger {
    let mut c = gen_constructor();
    c.get_thread_logger()
}

pub fn get_thread_runner() -> ThreadRunner {
    let c = gen_constructor();
    c.get_thread_runner()
}

struct ThreadResult {
    pub thread_name: String,
    pub res: Result<(), anyhow::Error>,
    panic_after_done: bool,
}

#[derive(Debug)]
struct ThreadConstructor {
    rx: Option<mpsc::Receiver<ThreadResult>>,
    tx: mpsc::Sender<ThreadResult>,
}

pub struct ThreadRunner {
    tx: mpsc::Sender<ThreadResult>,
}

pub struct ThreadLogger {
    rx: mpsc::Receiver<ThreadResult>,
}

impl ThreadConstructor {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { tx, rx: Some(rx) }
    }

    fn get_thread_runner(&self) -> ThreadRunner {
        ThreadRunner::new(self.tx.clone())
    }

    fn get_thread_logger(&mut self) -> ThreadLogger {
        let rx = self.rx.take().expect("thread logger already initialized");
        ThreadLogger::new(rx)
    }
}

impl ThreadLogger {
    fn new(rx: mpsc::Receiver<ThreadResult>) -> Self {
        Self { rx }
    }

    pub fn log_results(self) {
        let rx = self.rx;

        while let Ok(result) = rx.recv() {
            match result.res {
                Ok(_) => {
                    println!("Thread {} returned successfully", result.thread_name);
                }
                Err(err) => {
                    ratatui::restore();
                    println!("Error from thread {}: {}", result.thread_name, err);
                    println!("err: {}", err.backtrace());
                }
            };

            if result.panic_after_done {
                break;
            }
        }
    }
}

impl ThreadRunner {
    fn new(tx: mpsc::Sender<ThreadResult>) -> Self {
        Self { tx }
    }
    fn catch_thread_erros<F, T>(
        &self,
        tx: mpsc::Sender<ThreadResult>,
        thread_name: T,
        f: F,
        panic_after_done: bool,
    ) -> impl FnOnce()
    where
        F: FnOnce() -> Result<()>,
        T: AsRef<str>,
    {
        move || {
            let res = f();

            let res = ThreadResult {
                thread_name: thread_name.as_ref().into(),
                res,
                panic_after_done,
            };
            tx.send(res).expect("Listener for ThreatLogger dropped!!");
        }
    }

    pub fn spawn<F, T>(&self, thread_name: T, panic_after_done: bool, f: F)
    where
        F: FnOnce() -> Result<()> + Send + 'static,
        T: AsRef<str> + Send + 'static + Clone,
    {
        let tx = self.tx.clone();
        let thread_name_copy = thread_name.clone();
        let name = String::from(thread_name_copy.as_ref());

        // Move needed data into a new closure so that `self` is no longer needed
        let closure = move || -> ThreadResult {
            let res = f(); // Run the thread logic

            let result = ThreadResult {
                thread_name: thread_name_copy.as_ref().to_string(),
                res,
                panic_after_done,
            };

            // Optionally panic if required
            if result.panic_after_done && result.res.is_err() {
                panic!(
                    "Thread '{}' panicked with error: {}",
                    result.thread_name,
                    result.res.as_ref().unwrap_err()
                );
            }

            result
        };

        thread::Builder::new()
            .name(name)
            .spawn(closure)
            .expect(&format!("failed to build {} thread", thread_name.as_ref()));
    }
}
