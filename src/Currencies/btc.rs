use colored::*;
use core::{fmt, fmt::Display, str::FromStr};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Serialize};
use serde_json::from_str;
use fast_qr::convert::{image::ImageBuilder, Builder, Shape};
use fast_qr::qr::QRBuilder;
use std::sync::{Arc, Mutex};

use bitcoin::util::key::Secp256k1;

use crate::configuration::*;
use crate::configuration::application_settings::ApplicationSettings;

pub fn generate_from_private_key(private_key: &str) -> Option<BitcoinWallet> {
    match BitcoinWallet::from_private_key(private_key) {
        Ok(btc_wallet) => return Some(btc_wallet),
        Err(e) => {
            println!("{}", format!("ERROR: {:?}", e).red());
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
    pub private_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
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
    pub fn new() -> Result<Self, block_error::Error> {
        let s = Secp256k1::new();
        let (secp_private_key, secp_public_key) = s.generate_keypair(&mut bitcoin::secp256k1::rand::thread_rng());

        let public_key = bitcoin::PublicKey::new(secp_public_key);
        let private_key = bitcoin::PrivateKey::new(secp_private_key, bitcoin::network::constants::Network::Bitcoin);
        let address = bitcoin::util::address::Address::p2pkh(&public_key, bitcoin::network::constants::Network::Bitcoin);
        Ok(Self {
            private_key: Some(private_key.to_string()),
            public_key: Some(public_key.to_string()),
            address: Some(address.to_string()),
            network: Some(format!("{}", private_key.network)),
            compressed: Some(private_key.compressed),
            balance: Arc::new(Mutex::new(String::from("Uninitialized"))),
            ..Default::default()
        })
    }

    pub fn from_private_key(pk: &str) -> Result<Self, block_error::Error> {
        let s = Secp256k1::new();
        let private_key = bitcoin::PrivateKey::from_wif(pk)?;
        let public_key = private_key.public_key(&s);
        let address = bitcoin::util::address::Address::p2pkh(&public_key, bitcoin::network::constants::Network::Bitcoin);
        Ok(Self {
            private_key: Some(private_key.to_string()),
            public_key: Some(public_key.to_string()),
            address: Some(address.to_string()),
            network: Some(format!("{}", private_key.network)),
            compressed: Some(private_key.compressed),
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
