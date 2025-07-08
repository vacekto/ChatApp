use std::env;

use backtrace::Backtrace;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("Failed to serialize / deserialize using bincode, actual error: {0}{1}")]
pub struct BincodeErr(pub Box<bincode::ErrorKind>, pub Bt);

#[derive(Error, Debug)]
#[error("Failed to read or write framed message via TCP stream, actual error: {0}{1}")]
pub struct TcpErr(pub std::io::Error, pub Bt);

#[derive(Error, Debug)]
#[error("{}", self.location)]
pub struct Bt {
    location: String,
}

impl Bt {
    pub fn new() -> Self {
        Self {
            location: Bt::get_location(),
        }
    }

    // compactly writes out location where the Bt::new constructor was called
    fn get_location() -> String {
        let bt = Backtrace::new();

        let location = bt
            .frames()
            .iter()
            .skip(2)
            .flat_map(|frame| frame.symbols())
            .find_map(|symbol| {
                if let (Some(file), Some(line)) = (symbol.filename(), symbol.lineno()) {
                    Some((file.display().to_string(), line))
                } else {
                    None
                }
            });

        let whole_backtrace = match env::var("RUST_BACKTRACE") {
            Ok(value) if value == "1" => format!("\nbacktrace: \n{bt:?}"),
            _ => format!(""),
        };

        if let Some((file, line)) = location {
            format!("\nlocation: {file}:{line}{whole_backtrace}")
        } else {
            format!("(location unknown)")
        }
    }
}
