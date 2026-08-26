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

/// Human-readable rendering, for text that reaches the user.
///
/// `Debug` stays as it was, because the log format and several existing views depend on the
/// `ERROR: "...".` shape that `new` produces. `Display` is the clean form: no prefix, no
/// wrapping quotes, no trailing full stop. Views that currently strip those by hand can use
/// this instead.
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IOError(error) => write!(f, "{error}"),
            Error::GlibError(error) => write!(f, "{error}"),
            Error::UTF8Error(error) => write!(f, "{error}"),
            Error::Crate(which, message) => write!(f, "{which}: {message}"),
            Error::New(message) => {
                let trimmed = message
                    .trim_start_matches("ERROR: ")
                    .trim_end_matches('.')
                    .trim_matches('"');
                write!(f, "{trimmed}")
            }
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_strips_the_debug_wrapping_that_new_adds() {
        let error = Error::new("something went wrong".to_string());
        assert_eq!(format!("{error}"), "something went wrong");
        // Debug keeps its existing shape, which the log format and older views rely on.
        assert!(format!("{error:?}").contains("ERROR: "));
    }

    #[test]
    fn display_of_a_wrapped_crate_error_names_the_crate() {
        let error = Error::Crate("serde_json", "trailing comma".to_string());
        assert_eq!(format!("{error}"), "serde_json: trailing comma");
    }
}
