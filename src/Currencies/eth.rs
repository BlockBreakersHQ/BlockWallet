use wagyu_ethereum::*;
use wagyu_model::*;
use colored::*;
use core::{fmt, fmt::Display, str::FromStr};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Serialize};
use serde_json::Value;
use fast_qr::convert::{image::ImageBuilder, Builder, Shape};
use fast_qr::qr::QRBuilder;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use crate::configuration::*;
use crate::configuration::application_settings::ApplicationSettings;
use crate::currencies::transactions::*;
use crate::currencies::tokens::*;

pub fn generate_eth_hd_wallet() -> Option<EthereumWallet> {
    match EthereumWallet::new_hd::<wagyu_ethereum::network::Ropsten, wagyu_ethereum::wordlist::English, _>(
        &mut StdRng::from_entropy(),
        24,
        None,
        "m/44'/60'/0'/0'/0",
    ) {
        Ok(eth_wallet) => return Some(eth_wallet),
        Err(e) => {
            let path = ApplicationSettings::find_error_path().unwrap();
            ApplicationSettings::write_error_to_path(&path, format!("ERROR: {:?}", e));
            return None
        }
    };
}

pub fn generate_from_mnemonic(mnemonic: &str, mut path: &str) -> Option<EthereumWallet> {
    if path.is_empty() {
        path = "m/44'/60'/0'/0'/0";
    }

    match EthereumWallet::from_mnemonic::<wagyu_ethereum::network::Mainnet, wagyu_ethereum::wordlist::English>(
        mnemonic,
        None,
        path
    ) {
        Ok(eth_wallet) => return Some(eth_wallet),
        Err(e) => {
            let path = ApplicationSettings::find_error_path().unwrap();
            ApplicationSettings::write_error_to_path(&path, format!("ERROR: {:?}", e));
            return None
        }
    };
}

pub fn generate_from_private_key(private_key: &str) -> Option<EthereumWallet> {
    match EthereumWallet::from_private_key(private_key) {
        Ok(eth_wallet) => return Some(eth_wallet),
        Err(e) => {
            let path = ApplicationSettings::find_error_path().unwrap();
            ApplicationSettings::write_error_to_path(&path, format!("ERROR: {:?}", e));
            return None
        }
    }
}

pub fn generate_from_extended_private_key(extended_private_key: &str, path: &str) -> Option<EthereumWallet> {
    let path_option;
    if path.is_empty() {
        path_option = Some(String::from("m/44'/60'/0'/0'/0"));
    }
    else {
        path_option = Some(String::from(path));
    }

    match EthereumWallet::from_extended_private_key::<wagyu_ethereum::network::Mainnet>(extended_private_key, &path_option) {
        Ok(eth_wallet) => return Some(eth_wallet),
        Err(e) => {
            let path = ApplicationSettings::find_error_path().unwrap();
            ApplicationSettings::write_error_to_path(&path, format!("ERROR: {:?}", e));
            return None
        }
    }
}

#[derive(Serialize, Debug, Default, Clone)]
pub struct EthereumWallet {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    pub balance: Arc<Mutex<String>>,
    pub erc20_balances: Arc<Mutex<HashMap<String, f64>>>,
    pub transactions: Arc<Mutex<Vec<EthTransaction>>>,
    pub last_block: Arc<Mutex<i64>>,
    
}

impl EthereumWallet {
    pub fn new<R: Rng>(rng: &mut R) -> Result<Self, block_error::Error> {
        let private_key = EthereumPrivateKey::new(rng)?;
        let public_key = private_key.to_public_key();
        let address = public_key.to_address(&EthereumFormat::Standard)?;
        Ok(Self {
            private_key: Some(private_key.to_string()),
            public_key: Some(public_key.to_string()),
            address: Some(address.to_string()),
            balance: Arc::new(Mutex::new(String::from("Uninitialized"))),
            erc20_balances: Arc::new(Mutex::new(HashMap::new())),
            transactions: Arc::new(Mutex::new(Vec::new())),
            last_block: Arc::new(Mutex::new(0)),
            ..Default::default()
        })
    }

    pub fn new_hd<N: EthereumNetwork, W: EthereumWordlist, R: Rng>(
        rng: &mut R,
        word_count: u8,
        password: Option<&str>,
        path: &str,
    ) -> Result<Self, block_error::Error> {
        let mnemonic = EthereumMnemonic::<N, W>::new_with_count(rng, word_count)?;
        let master_extended_private_key = mnemonic.to_extended_private_key(password)?;
        let derivation_path = EthereumDerivationPath::from_str(path)?;
        let extended_private_key = master_extended_private_key.derive(&derivation_path)?;
        let extended_public_key = extended_private_key.to_extended_public_key();
        let private_key = extended_private_key.to_private_key();
        let public_key = extended_public_key.to_public_key();
        let address = public_key.to_address(&EthereumFormat::Standard)?;
        Ok(Self {
            path: Some(path.to_string()),
            password: password.map(String::from),
            mnemonic: Some(mnemonic.to_string()),
            extended_private_key: Some(extended_private_key.to_string()),
            extended_public_key: Some(extended_public_key.to_string()),
            private_key: Some(private_key.to_string()),
            public_key: Some(public_key.to_string()),
            address: Some(address.to_string()),
            balance: Arc::new(Mutex::new(String::from("Uninitialized"))),
            erc20_balances: Arc::new(Mutex::new(HashMap::new())),
            transactions: Arc::new(Mutex::new(Vec::new())),
            last_block: Arc::new(Mutex::new(0)),
            ..Default::default()
        })
    }

    pub fn from_mnemonic<N: EthereumNetwork, W: EthereumWordlist>(
        mnemonic: &str,
        password: Option<&str>,
        path: &str,
    ) -> Result<Self, block_error::Error> {
        let mnemonic = EthereumMnemonic::<N, W>::from_phrase(&mnemonic)?;
        let master_extended_private_key = mnemonic.to_extended_private_key(password)?;
        let derivation_path = EthereumDerivationPath::from_str(path)?;
        let extended_private_key = master_extended_private_key.derive(&derivation_path)?;
        let extended_public_key = extended_private_key.to_extended_public_key();
        let private_key = extended_private_key.to_private_key();
        let public_key = extended_public_key.to_public_key();
        let address = public_key.to_address(&EthereumFormat::Standard)?;
        Ok(Self {
            path: Some(path.to_string()),
            password: password.map(String::from),
            mnemonic: Some(mnemonic.to_string()),
            extended_private_key: Some(extended_private_key.to_string()),
            extended_public_key: Some(extended_public_key.to_string()),
            private_key: Some(private_key.to_string()),
            public_key: Some(public_key.to_string()),
            address: Some(address.to_string()),
            balance: Arc::new(Mutex::new(String::from("Uninitialized"))),
            erc20_balances: Arc::new(Mutex::new(HashMap::new())),
            transactions: Arc::new(Mutex::new(Vec::new())),
            last_block: Arc::new(Mutex::new(0)),
            ..Default::default()
        })
    }

    pub fn from_extended_private_key<N: EthereumNetwork>(
        extended_private_key: &str,
        path: &Option<String>
    ) -> Result<Self, block_error::Error> {
        let mut extended_private_key = EthereumExtendedPrivateKey::<N>::from_str(extended_private_key)?;
        if let Some(derivation_path) = path {
            let derivation_path = EthereumDerivationPath::from_str(&derivation_path)?;
            extended_private_key = extended_private_key.derive(&derivation_path)?;
        }
        let extended_public_key = extended_private_key.to_extended_public_key();
        let private_key = extended_private_key.to_private_key();
        let public_key = extended_public_key.to_public_key();
        let address = public_key.to_address(&EthereumFormat::Standard)?;
        Ok(Self {
            path: path.clone(),
            extended_private_key: Some(extended_private_key.to_string()),
            extended_public_key: Some(extended_public_key.to_string()),
            private_key: Some(private_key.to_string()),
            public_key: Some(public_key.to_string()),
            address: Some(address.to_string()),
            balance: Arc::new(Mutex::new(String::from("Uninitialized"))),
            erc20_balances: Arc::new(Mutex::new(HashMap::new())),
            transactions: Arc::new(Mutex::new(Vec::new())),
            last_block: Arc::new(Mutex::new(0)),
            ..Default::default()
        })
    }

    pub fn from_private_key(private_key: &str) -> Result<Self, block_error::Error> {
        let private_key = EthereumPrivateKey::from_str(private_key)?;
        let public_key = private_key.to_public_key();
        let address = public_key.to_address(&EthereumFormat::Standard)?;
        Ok(Self {
            private_key: Some(private_key.to_string()),
            public_key: Some(public_key.to_string()),
            address: Some(address.to_string()),
            balance: Arc::new(Mutex::new(String::from("Uninitialized"))),
            erc20_balances: Arc::new(Mutex::new(HashMap::new())),
            transactions: Arc::new(Mutex::new(Vec::new())),
            last_block: Arc::new(Mutex::new(0)),
            ..Default::default()
        })
    }

    pub async fn get_balance(address: String, etherscan_key: String) -> Option<String> {
        let etherscan_get_address_balance_url = 
            format!("https://api.etherscan.io/api\
            ?module=account\
            &action=balance\
            &address={}\
            &tag=latest\
            &apikey={}", address, etherscan_key);

        let resp = match reqwest::get(etherscan_get_address_balance_url).await {
            Ok(resp) => resp,
            Err(_) => return Some(String::from("Uninitialized"))
        };

        let text = match resp.text().await {
            Ok(text) => text,
            Err(_) => return Some(String::from("Uninitialized"))
        };

        let json: Value = match serde_json::from_str(&text) {
            Ok(r)  => r,
            Err(_) => return Some(String::from("Uninitialized"))
        };

        return Some(json["result"].to_string().replace("\"", ""));
    }

    pub async fn get_erc20_balances(&mut self, etherscan_key: String, tokens: HashMap<String, Token>) {
        let orig_address = match &self.address {
            Some(address) => address,
            None => return
        };
        let eth_transaction_url = format!("https://api.etherscan.io/api\
            ?module=account\
            &action=tokentx\
            &address={}\
            &startblock={}\
            &apikey={}", orig_address, &*self.last_block.lock().unwrap(), etherscan_key.clone());

        let resp = match reqwest::get(eth_transaction_url).await {
            Ok(resp) => resp,
            Err(_) => return
        };

        let text = match resp.text().await {
            Ok(text) => text,
            Err(_) => return
        };

        let json: Value = match serde_json::from_str(&text) {
            Ok(r)  => r,
            Err(_) => return
        };

        let eth_transactions: Vec<EthTransaction> = match serde_json::from_str(&json["result"].to_string()) {
            Ok(eth_transactions) => eth_transactions,
            Err(e) => panic!("Error parsing eth_transactions: {}", e)
        };

        if eth_transactions.len() > 0 {
            *self.last_block.lock().unwrap() = match &eth_transactions[eth_transactions.len() - 1].blockNumber {
                Some(block_number) => block_number.parse::<i64>().expect("Not a number!"),
                None => 0
            };
        } else {
            *self.last_block.lock().unwrap() = 0;
        }
        

        let address = orig_address.to_uppercase();

        for eth_transaction in &eth_transactions {
            let symbol = match &eth_transaction.tokenSymbol {
                Some(symbol) => symbol,
                None => continue
            };
            if tokens.contains_key(symbol) {
                self.transactions.lock().unwrap().push(eth_transaction.clone());
                let to = match &eth_transaction.to {
                    Some(to) => to.to_uppercase(),
                    None => continue
                };

                let from = match &eth_transaction.from {
                    Some(from) => from.to_uppercase(),
                    None => continue
                };

                if self.erc20_balances.lock().unwrap().contains_key(&eth_transaction.tokenSymbol.clone().unwrap()) {
                    if to == address {
                        let mut balance = self.erc20_balances.lock().unwrap()[&eth_transaction.tokenSymbol.clone().unwrap()];
                        let value = match eth_transaction.value.clone() {
                            Some(value) => value,
                            None => continue
                        };
                        let current_balance: f64 = value.parse::<f64>().expect("Not a number!");
                        balance += current_balance;
                        self.erc20_balances.lock().unwrap().insert(eth_transaction.tokenSymbol.clone().unwrap(), balance);
                    } else if from == address {
                        let mut balance = self.erc20_balances.lock().unwrap()[&eth_transaction.tokenSymbol.clone().unwrap()];
                        let value = match eth_transaction.value.clone() {
                            Some(value) => value,
                            None => continue
                        };
                        let current_balance: f64 = value.parse::<f64>().expect("Not a number!");
                        balance -= current_balance;
                        self.erc20_balances.lock().unwrap().insert(eth_transaction.tokenSymbol.clone().unwrap(), balance);
                    }
                } else {
                    self.erc20_balances.lock().unwrap().insert(eth_transaction.tokenSymbol.clone().unwrap(), 0.0);
                    if to == address {
                        let mut balance = self.erc20_balances.lock().unwrap()[&eth_transaction.tokenSymbol.clone().unwrap()];
                        let value = match eth_transaction.value.clone() {
                            Some(value) => value,
                            None => continue
                        };
                        let current_balance: f64 = value.parse::<f64>().expect("Not a number!");
                        balance += current_balance;
                        self.erc20_balances.lock().unwrap().insert(eth_transaction.tokenSymbol.clone().unwrap(), balance);
                    } else if from == address {
                        let mut balance = self.erc20_balances.lock().unwrap()[&eth_transaction.tokenSymbol.clone().unwrap()];
                        let value = match eth_transaction.value.clone() {
                            Some(value) => value,
                            None => continue
                        };
                        let current_balance: f64 = value.parse::<f64>().expect("Not a number!");
                        balance -= current_balance;
                        self.erc20_balances.lock().unwrap().insert(eth_transaction.tokenSymbol.clone().unwrap(), balance);
                    }
                }
            }
        }
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
impl Display for EthereumWallet {
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
            match &self.network {
                Some(network) => format!("      {}              {}\n", "Network".cyan().bold(), network),
                _ => "".to_owned(),
            },
        ]
        .concat();

        let output = output[..output.len() - 1].to_owned();
        write!(f, "\n{}", output)
    }
}