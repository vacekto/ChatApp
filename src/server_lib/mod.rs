pub fn log(err: Box<dyn std::error::Error>, msg: Option<&str>) {
    println!(
        "an errrrrr occurrrrrrrrred: {}, msg: {}",
        err,
        match msg {
            Some(msg) => msg,
            None => "",
        }
    );
}
