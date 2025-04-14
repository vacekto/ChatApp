pub const DEFUALT_HOSTNAME: &str = "localhost";
pub const DEFUALT_PORT: &str = "11111";

pub fn log(err: anyhow::Error, msg: Option<&str>) {
    println!(
        "an errrrrr occurrrrrrrrred: {}, msg: {}",
        err,
        match msg {
            Some(msg) => msg,
            None => "",
        }
    );
}
