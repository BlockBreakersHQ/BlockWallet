use wagyu_bitcoin::*;
use wagyu_model::*;
use colored::*;
use core::{fmt, fmt::Display, str::FromStr};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Serialize};
use serde_json::from_str;
use fast_qr::convert::{image::ImageBuilder, Builder, Shape};
use fast_qr::qr::QRBuilder;
use std::sync::{Arc, Mutex};

use crate::configuration::*;
use crate::configuration::application_settings::ApplicationSettings;

pub fn generate_btc_hd_wallet() -> Option<BitcoinWallet> {
    match BitcoinWallet::new_hd::<wagyu_bitcoin::network::Mainnet, wagyu_bitcoin::wordlist::English, _>(
        &mut StdRng::from_entropy(),
        24,
        None,
        "m/44'/60'/0'/0'/0",
    ) {
        Ok(btc_wallet) => return Some(btc_wallet),
        Err(e) => {
            let path = ApplicationSettings::find_error_path().unwrap();
            ApplicationSettings::write_error_to_path(&path, format!("ERROR: {:?}", e));
            return None
        }
    };
}

pub fn generate_from_mnemonic(mnemonic: &str, mut path: &str) -> Option<BitcoinWallet> {
    if path.is_empty() {
        path = "m/44'/60'/0'/0'/0";
    }

    match BitcoinWallet::from_mnemonic::<wagyu_bitcoin::network::Mainnet, wagyu_bitcoin::wordlist::English>(
        mnemonic,
        &None,
        path
    ) {
        Ok(btc_wallet) => return Some(btc_wallet),
        Err(e) => {
            let path = ApplicationSettings::find_error_path().unwrap();
            ApplicationSettings::write_error_to_path(&path, format!("ERROR: {:?}", e));
            return None
        }
    };
}

pub fn generate_from_private_key(private_key: &str) -> Option<BitcoinWallet> {
    match BitcoinWallet::from_private_key::<wagyu_bitcoin::network::Mainnet>(private_key, &BitcoinFormat::Bech32) {
        Ok(btc_wallet) => return Some(btc_wallet),
        Err(e) => {
            let path = ApplicationSettings::find_error_path().unwrap();
            ApplicationSettings::write_error_to_path(&path, format!("ERROR: {:?}", e));
            return None
        }
    }
}

pub fn generate_from_extended_private_key(extended_private_key: &str, path: &str) -> Option<BitcoinWallet> {
    let path_option;
    if path.is_empty() {
        path_option = Some(String::from("m/44'/60'/0'/0'/0"));
    }
    else {
        path_option = Some(String::from(path));
    }

    match BitcoinWallet::from_extended_private_key::<wagyu_bitcoin::network::Mainnet>(extended_private_key, &path_option) {
        Ok(btc_wallet) => return Some(btc_wallet),
        Err(e) => {
            let path = ApplicationSettings::find_error_path().unwrap();
            ApplicationSettings::write_error_to_path(&path, format!("ERROR: {:?}", e));
            return None
        }
    }
}

#[derive(Serialize, Debug, Default, Clone)]
pub struct BitcoinWallet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mnemonic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extended_private_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extended_public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compressed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<String>,
    pub balance: Arc<Mutex<String>>,
}

impl BitcoinWallet {
    pub fn new<N: BitcoinNetwork, R: Rng>(rng: &mut R, format: &BitcoinFormat) -> Result<Self, block_error::Error> {
        let private_key = BitcoinPrivateKey::<N>::new(rng)?;
        let public_key = private_key.to_public_key();
        let address = public_key.to_address(format)?;
        Ok(Self {
            private_key: Some(private_key.to_string()),
            public_key: Some(public_key.to_string()),
            address: Some(address.to_string()),
            network: Some(N::NAME.to_string()),
            format: Some(address.format().to_string()),
            compressed: private_key.is_compressed().into(),
            balance: Arc::new(Mutex::new(String::from("Uninitialized"))),
            ..Default::default()
        })
    }

    pub fn new_hd<N: BitcoinNetwork, W: BitcoinWordlist, R: Rng>(
        rng: &mut R,
        word_count: u8,
        password: Option<&str>,
        path: &str,
    ) -> Result<Self, block_error::Error> {
        let mnemonic = BitcoinMnemonic::<N, W>::new_with_count(rng, word_count)?;
        let master_extended_private_key = mnemonic.to_extended_private_key(password)?;
        let derivation_path = BitcoinDerivationPath::from_str(path)?;
        let extended_private_key = master_extended_private_key.derive(&derivation_path)?;
        let extended_public_key = extended_private_key.to_extended_public_key();
        let private_key = extended_private_key.to_private_key();
        let public_key = extended_public_key.to_public_key();
        let address = public_key.to_address(&extended_private_key.format())?;
        let compressed = private_key.is_compressed();
        Ok(Self {
            path: Some(path.to_string()),
            password: password.map(String::from),
            mnemonic: Some(mnemonic.to_string()),
            extended_private_key: Some(extended_private_key.to_string()),
            extended_public_key: Some(extended_public_key.to_string()),
            private_key: Some(private_key.to_string()),
            public_key: Some(public_key.to_string()),
            address: Some(address.to_string()),
            format: Some(address.format().to_string()),
            network: Some(N::NAME.to_string()),
            compressed: Some(compressed),
            balance: Arc::new(Mutex::new(String::from("Uninitialized"))),
            ..Default::default()
        })
    }

    pub fn from_mnemonic<N: BitcoinNetwork, W: BitcoinWordlist>(
        mnemonic: &str,
        password: &Option<&str>,
        path: &str,
    ) -> Result<Self, block_error::Error> {
        let mnemonic = BitcoinMnemonic::<N, W>::from_phrase(&mnemonic)?;
        let master_extended_private_key = mnemonic.to_extended_private_key(password.clone())?;
        let derivation_path = BitcoinDerivationPath::from_str(path)?;
        let extended_private_key = master_extended_private_key.derive(&derivation_path)?;
        let extended_public_key = extended_private_key.to_extended_public_key();
        let private_key = extended_private_key.to_private_key();
        let public_key = extended_public_key.to_public_key();
        let address = public_key.to_address(&extended_private_key.format())?;
        let compressed = private_key.is_compressed();
        Ok(Self {
            path: Some(path.to_string()),
            password: password.map(String::from),
            mnemonic: Some(mnemonic.to_string()),
            extended_private_key: Some(extended_private_key.to_string()),
            extended_public_key: Some(extended_public_key.to_string()),
            private_key: Some(private_key.to_string()),
            public_key: Some(public_key.to_string()),
            address: Some(address.to_string()),
            format: Some(address.format().to_string()),
            network: Some(N::NAME.to_string()),
            compressed: Some(compressed),
            balance: Arc::new(Mutex::new(String::from("Uninitialized"))),
            ..Default::default()
        })
    }

    pub fn from_extended_private_key<N: BitcoinNetwork>(
        extended_private_key: &str,
        path: &Option<String>,
    ) -> Result<Self, block_error::Error> {
        let mut extended_private_key = BitcoinExtendedPrivateKey::<N>::from_str(extended_private_key)?;
        if let Some(derivation_path) = path {
            let derivation_path = BitcoinDerivationPath::from_str(&derivation_path)?;
            extended_private_key = extended_private_key.derive(&derivation_path)?;
        }
        let extended_public_key = extended_private_key.to_extended_public_key();
        let private_key = extended_private_key.to_private_key();
        let public_key = extended_public_key.to_public_key();
        let address = public_key.to_address(&extended_private_key.format())?;
        let compressed = private_key.is_compressed();
        Ok(Self {
            path: path.clone(),
            extended_private_key: Some(extended_private_key.to_string()),
            extended_public_key: Some(extended_public_key.to_string()),
            private_key: Some(private_key.to_string()),
            public_key: Some(public_key.to_string()),
            address: Some(address.to_string()),
            format: Some(address.format().to_string()),
            network: Some(N::NAME.to_string()),
            compressed: Some(compressed),
            balance: Arc::new(Mutex::new(String::from("Uninitialized"))),
            ..Default::default()
        })
    }

    pub fn from_private_key<N: BitcoinNetwork>(private_key: &str, format: &BitcoinFormat) -> Result<Self, block_error::Error> {
        let private_key = BitcoinPrivateKey::<N>::from_str(private_key)?;
        let public_key = private_key.to_public_key();
        let address = public_key.to_address(format)?;
        Ok(Self {
            private_key: Some(private_key.to_string()),
            public_key: Some(public_key.to_string()),
            address: Some(address.to_string()),
            network: Some(N::NAME.to_string()),
            format: Some(address.format().to_string()),
            compressed: private_key.is_compressed().into(),
            balance: Arc::new(Mutex::new(String::from("Uninitialized"))),
            ..Default::default()
        })
    }

    pub async fn get_balance(address: String) -> Option<String> {
        let balance_endpoint = format!("https://blockchain.info/q/getreceivedbyaddress/{}", address);
        let resp = match reqwest::get(balance_endpoint).await.ok()?.text().await {
            Ok(r)  => r,
            Err(_) => return None
        };

        let mut balance = match from_str::<f64>(&resp) {
            Ok(b)  => b,
            Err(_) => return None
        };

        if balance > 0.0 {
            balance = balance / 100000000.0;
        }
        return Some(balance.to_string());
    }

    pub fn set_wallet_name(&mut self, name: String) {
        self.wallet_name = Some(name);
    }

    pub fn generate_qr_address(&self) -> Result<gdk4::Texture, block_error::Error> {
        let address = match &self.address {
            Some(addr) => addr,
            None => ""
        };

        let qrcode = QRBuilder::new(address.to_string())
            .build()
            .unwrap();

        let img = ImageBuilder::default()
            .shape(Shape::RoundedSquare)
            .fit_width(300)
            .to_pixmap(&qrcode);

        let encoded_png = match img.encode_png() {
            Ok(png) => png,
            Err(e)  => return Err(block_error::Error::IOError(e.into()))
        };

        let texture = gdk4::Texture::from_bytes(&glib::Bytes::from(&encoded_png))?;
        Ok(texture)
    }
}

#[cfg_attr(tarpaulin, skip)]
impl Display for BitcoinWallet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let output = [
            match &self.wallet_name {
                Some(wallet_name) => format!("      {}          {}\n", "Wallet Name".cyan().bold(), wallet_name),
                _ => "".to_owned(),
            },
            match &self.path {
                Some(path) => format!("      {}                 {}\n", "Path".cyan().bold(), path),
                _ => "".to_owned(),
            },
            match &self.password {
                Some(password) => format!("      {}             {}\n", "Password".cyan().bold(), password),
                _ => "".to_owned(),
            },
            match &self.mnemonic {
                Some(mnemonic) => format!("      {}             {}\n", "Mnemonic".cyan().bold(), mnemonic),
                _ => "".to_owned(),
            },
            match &self.extended_private_key {
                Some(extended_private_key) => format!(
                    "      {} {}\n",
                    "Extended Private Key".cyan().bold(),
                    extended_private_key
                ),
                _ => "".to_owned(),
            },
            match &self.extended_public_key {
                Some(extended_public_key) => format!(
                    "      {}  {}\n",
                    "Extended Public Key".cyan().bold(),
                    extended_public_key
                ),
                _ => "".to_owned(),
            },
            match &self.private_key {
                Some(private_key) => format!("      {}          {}\n", "Private Key".cyan().bold(), private_key),
                _ => "".to_owned(),
            },
            match &self.public_key {
                Some(public_key) => format!("      {}           {}\n", "Public Key".cyan().bold(), public_key),
                _ => "".to_owned(),
            },
            match &self.address {
                Some(address) => format!("      {}              {}\n", "Address".cyan().bold(), address),
                _ => "".to_owned(),
            },
            match &self.format {
                Some(format) => format!("      {}               {}\n", "Format".cyan().bold(), format),
                _ => "".to_owned(),
            },
            match &self.network {
                Some(network) => format!("      {}              {}\n", "Network".cyan().bold(), network),
                _ => "".to_owned(),
            },
            match &self.compressed {
                Some(compressed) => format!("      {}           {}\n", "Compressed".cyan().bold(), compressed),
                _ => "".to_owned(),
            },
            match &self.transaction_id {
                Some(transaction_id) => format!("      {}       {}\n", "Transaction Id".cyan().bold(), transaction_id),
                _ => "".to_owned(),
            },
            match &self.transaction_hex {
                Some(transaction_hex) => {
                    format!("      {}      {}\n", "Transaction Hex".cyan().bold(), transaction_hex)
                }
                _ => "".to_owned(),
            },
        ]
        .concat();

        // Removes final new line character
        let output = output[..output.len() - 1].to_owned();
        write!(f, "\n{}", output)
    }
}
