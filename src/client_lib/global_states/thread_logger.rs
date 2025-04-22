use std::{
    sync::{mpsc, Mutex},
    thread,
};

use anyhow::{self, Result};
use once_cell::sync::OnceCell;

static GLOBAL: OnceCell<Mutex<ThreadConstructor>> = OnceCell::new();

fn gen_constructor() -> std::sync::MutexGuard<'static, ThreadConstructor> {
    match GLOBAL.get() {
        Some(c) => c.lock().unwrap_or_else(|e| e.into_inner()),
        None => {
            GLOBAL.set(Mutex::new(ThreadConstructor::new())).unwrap();
            GLOBAL
                .get()
                .unwrap()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
        }
    }
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

    pub fn log_results<T>(self, mut writer: T, panic: bool)
    where
        T: std::io::Write,
    {
        let rx = self.rx;

        let result = rx
            .recv()
            .expect("no transmitter alive, costructor and all runners dropped");

        let msg = match result.res {
            Ok(_) => format!("Thread {} returned successfully", result.thread_name),
            Err(err) => format!(
                "Thread \"{}\" returned with an error: {}.  \nBacktrace:\n {}",
                result.thread_name,
                err,
                err.backtrace()
            ),
        };

        writer.write(msg.as_bytes()).expect("failed to write log");
        writer.flush().unwrap();
        if panic {
            panic!("thread {} returned, panicking now.", result.thread_name);
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
            };
            tx.send(res).expect("Listener for ThreatLogger dropped!!");
        }
    }

    pub fn run<F, T>(&self, thread_name: T, f: F)
    where
        F: FnOnce() -> Result<()> + Send + 'static,
        T: AsRef<str> + Send + 'static + Clone,
    {
        let tx = self.tx.clone();
        let name = String::from(thread_name.as_ref());

        thread::Builder::new()
            .name(name)
            .spawn(self.catch_thread_erros(tx, thread_name.clone(), f))
            .expect(&format!("failed to buid {} thread", thread_name.as_ref()));
    }
}
