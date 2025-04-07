use std::env;

pub const BUFF_LENGTH: usize = 1024;

pub fn get_addr(default_hostname: &str, default_port: &str) -> String {
    let mut args = env::args();

    let hostname = match args.nth(1) {
        Some(h) => h,
        None => String::from(default_hostname),
    };

    let port = match args.nth(2) {
        Some(p) => p,
        None => String::from(default_port),
    };

    hostname + ":" + &port
}
