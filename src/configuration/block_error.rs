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
        Error::Crate("request", describe_request_error(&error))
    }
}

/// Turn a transport failure into one short line a person can act on.
///
/// This used to be `format!("{:?}", error)`, which Debug-formats the whole `reqwest::Error`:
/// the request struct, the hyper source beneath it, and, for a TLS failure, the entire
/// OpenSSL error stack with file names and line numbers. On a 360 px phone screen that filled
/// the swap results with a wall of text and buried the one fact that mattered, which was that
/// a gateway's certificate had expired.
///
/// Only the host is named, never the full URL. Several of these URLs carry the user's own
/// address in the query string, and an error message is not a good place for it to surface.
fn describe_request_error(error: &reqwest::Error) -> String {
    let host = error
        .url()
        .and_then(|url| url.host_str())
        .unwrap_or("the server")
        .to_string();

    if error.is_timeout() {
        return format!("{host} did not respond in time");
    }
    if let Some(status) = error.status() {
        return format!("{host} returned HTTP {}", status.as_u16());
    }

    // reqwest exposes no predicate for "the certificate was bad", so the cause chain is
    // walked and matched on. Flattened to lower case once rather than per pattern.
    let mut causes = String::new();
    let mut source = std::error::Error::source(error);
    while let Some(cause) = source {
        causes.push_str(&cause.to_string());
        causes.push(' ');
        source = cause.source();
    }
    // The precise reason a certificate was rejected (`X509VerifyResult { error: "certificate
    // has expired" }`) appears only in the Debug representation, not in any Display in the
    // cause chain. It is read here purely to classify, and never emitted: the Debug string is
    // the multi-line dump this function exists to replace.
    causes.push_str(&format!("{error:?}"));
    let causes = causes.to_lowercase();

    if causes.contains("certificate") || causes.contains("tls") || causes.contains("ssl") {
        if causes.contains("expired") {
            return format!("{host} has an expired security certificate");
        }
        if causes.contains("self-signed") || causes.contains("self signed") {
            return format!("{host} has a self-signed security certificate");
        }
        return format!("{host} has an invalid security certificate");
    }
    if causes.contains("dns") || causes.contains("resolve") || causes.contains("name or service") {
        return format!("{host} could not be found");
    }
    if error.is_connect() {
        return format!("could not connect to {host}");
    }
    if error.is_body() || error.is_decode() {
        return format!("{host} sent a reply that could not be read");
    }
    format!("could not reach {host}")
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
