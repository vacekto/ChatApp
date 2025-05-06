// channel bounds
pub const DIRECT_CAPACITY: usize = 30;
pub const ROOM_CAPACITY: usize = 30;
pub const COMM_CLIENT_CAPACITY: usize = 30;
pub const CLIENT_COMM_CAPACITY: usize = 30;
pub const CLIENT_MANAGER_CAPACITY: usize = 30;
pub const MANAGER_CLIENT_CAPACITY: usize = 10;

pub fn log(err: anyhow::Error, msg: Option<&str>) {
    println!("Server error occurred: {}", err);
    if let Some(msg) = msg {
        println!("msg: {}", msg);
        // println!("backtrace: {}", err.backtrace());
    }
}
