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
