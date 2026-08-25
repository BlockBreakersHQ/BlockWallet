#[derive(Debug)]
pub enum Error {
    IOError(std::io::Error),
    GlibError(glib::Error),
    UTF8Error(std::str::Utf8Error),
    Crate(&'static str, String),
    New(String),
}

impl From<std::io::Error> for Error {
    fn from (error: std::io::Error) -> Self {
        Error::IOError(error)
    }
}

impl From<glib::Error> for Error {
    fn from (error: glib::Error) -> Self {
        Error::GlibError(error)
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from (error: std::str::Utf8Error) -> Self {
        Error::UTF8Error(error)
    }
}

impl From<core::num::ParseIntError> for Error {
    fn from(error: core::num::ParseIntError) -> Self {
        Error::Crate("parse_int", format!("{:?}", error))
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(error: serde_json::error::Error) -> Self {
        Error::Crate("serde_json", format!("{:?}", error))
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Error::Crate("request", format!("{:?}", error))
    }
}

impl From<rustc_hex::FromHexError> for Error {
    fn from(error: rustc_hex::FromHexError) -> Self {
        Error::Crate("hex", format!("{:?}", error))
    }
}

impl From<alloy::signers::local::LocalSignerError> for Error {
    fn from(error: alloy::signers::local::LocalSignerError) -> Self {
        Error::Crate("alloy", format!("{:?}", error))
    }
}

impl Error {
    pub fn new(error: String) -> Self {
        Error::New(format!("ERROR: {:?}.", error))
    }
}
