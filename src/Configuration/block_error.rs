use wagyu_model::*;

#[derive(Debug)]
pub enum Error {
    IOError(std::io::Error),
    GlibError(glib::Error),
    UTF8Error(std::str::Utf8Error),
    AddressError(AddressError),
    AmountError(AmountError),
    Crate(&'static str, String),
    DerivationPathError(DerivationPathError),
    ExtendedPrivateKeyError(ExtendedPrivateKeyError),
    ExtendedPublicKeyError(ExtendedPublicKeyError),
    PrivateKeyError(PrivateKeyError),
    PublicKeyError(PublicKeyError),
    MnemonicError(MnemonicError),
    TransactionError(TransactionError),
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

impl From<AddressError> for Error {
    fn from(error: AddressError) -> Self {
        Error::AddressError(error)
    }
}

impl From<AmountError> for Error {
    fn from(error: AmountError) -> Self {
        Error::AmountError(error)
    }
}

impl From<core::num::ParseIntError> for Error {
    fn from(error: core::num::ParseIntError) -> Self {
        Error::Crate("parse_int", format!("{:?}", error))
    }
}

impl From<DerivationPathError> for Error {
    fn from(error: DerivationPathError) -> Self {
        Error::DerivationPathError(error)
    }
}

impl From<ExtendedPrivateKeyError> for Error {
    fn from(error: ExtendedPrivateKeyError) -> Self {
        Error::ExtendedPrivateKeyError(error)
    }
}

impl From<ExtendedPublicKeyError> for Error {
    fn from(error: ExtendedPublicKeyError) -> Self {
        Error::ExtendedPublicKeyError(error)
    }
}

impl From<hex::FromHexError> for Error {
    fn from(error: hex::FromHexError) -> Self {
        Error::Crate("hex", format!("{:?}", error))
    }
}

impl From<MnemonicError> for Error {
    fn from(error: MnemonicError) -> Self {
        Error::MnemonicError(error)
    }
}

impl From<PrivateKeyError> for Error {
    fn from(error: PrivateKeyError) -> Self {
        Error::PrivateKeyError(error)
    }
}

impl From<PublicKeyError> for Error {
    fn from(error: PublicKeyError) -> Self {
        Error::PublicKeyError(error)
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(error: serde_json::error::Error) -> Self {
        Error::Crate("serde_json", format!("{:?}", error))
    }
}

impl From<TransactionError> for Error {
    fn from(error: TransactionError) -> Self {
        Error::TransactionError(error)
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