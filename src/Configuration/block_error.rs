use ethers::prelude::*;

use crate::configuration::block_error::signer::SignerMiddlewareError;

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

impl From<cocoon::Error> for Error {
    fn from(error: cocoon::Error) -> Self {
        Error::Crate("cocoon", format!("{:?}", error))
    }
}

impl From<rustc_hex::FromHexError> for Error {
    fn from(error: rustc_hex::FromHexError) -> Self {
        Error::Crate("cocoon", format!("{:?}", error))
    }
}

impl From<bitcoin::util::key::Error> for Error {
    fn from(error: bitcoin::util::key::Error) -> Self {
        Error::Crate("bitcoin", format!("{:?}", error))
    }
}

impl From<WalletError> for Error {
    fn from(error: WalletError) -> Self {
        Error::Crate("ethers", format!("{:?}", error))
    }
}

impl From<ethers::providers::ProviderError> for Error {
    fn from(error: ethers::providers::ProviderError) -> Self {
        Error::Crate("ethers", format!("{:?}", error))
    }
}

impl From<ethers::utils::ConversionError> for Error {
    fn from(error: ethers::utils::ConversionError) -> Self {
        Error::Crate("ethers", format!("{:?}", error))
    }
}

impl From<SignerMiddlewareError<ethers::providers::Provider<Http>, Wallet<ethers::core::k256::ecdsa::SigningKey>>> for Error {
    fn from(error: SignerMiddlewareError<ethers::providers::Provider<Http>, Wallet<ethers::core::k256::ecdsa::SigningKey>>) -> Self {
        Error::Crate("ethers", format!("{:?}", error))
    }
}

impl Error {
    pub fn new(error: String) -> Self {
        Error::New(format!("ERROR: {:?}.", error))
    }
}