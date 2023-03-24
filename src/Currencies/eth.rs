use colored::*;
use core::{fmt, fmt::Display};
use serde::{Serialize};
use serde_json::Value;
use fast_qr::convert::{image::ImageBuilder, Builder, Shape};
use fast_qr::qr::QRBuilder;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use ethers::{
    core::{types::TransactionRequest, utils::Anvil, k256::ecdsa::SigningKey},
    providers::{Http, Middleware, Provider},
    utils,
    prelude::*,
    signers::{coins_bip39::English, MnemonicBuilder},
};

use bip39::{Language, Mnemonic, MnemonicType};

use crate::configuration::*;
use crate::configuration::application_settings::ApplicationSettings;
use crate::currencies::transactions::*;
use crate::currencies::tokens::*;
use crate::currencies::currency_pairs::*;

pub fn generate_eth_basic_wallet() -> Option<EthereumWallet> {
    match EthereumWallet::new() {
        Ok(eth_wallet) => return Some(eth_wallet),
        Err(e) => {
            let path = ApplicationSettings::find_error_path().unwrap();
            ApplicationSettings::write_error_to_path(&path, format!("ERROR: {:?}", e));
            return None
        }
    };
}

pub fn generate_eth_hd_wallet() -> Option<EthereumWallet> {
    match EthereumWallet::new_hd(
        24,
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

    match EthereumWallet::from_mnemonic(mnemonic, path) {
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

    match EthereumWallet::from_extended_private_key(extended_private_key, &path_option) {
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
    //#[serde(skip_serializing_if = "Option::is_none")]
    //pub extended_private_key: Option<String>,
    //#[serde(skip_serializing_if = "Option::is_none")]
    //pub extended_public_key: Option<String>,
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
    pub fn new() -> Result<Self, block_error::Error> {
        Ok(Self {
            address: Some(String::from("0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5")),
            ..Default::default()
        })
    }

    pub fn new_hd(word_count: u8, path: &str) -> Result<Self, block_error::Error> {
        let words = match word_count {
            12 => MnemonicType::Words12,
            15 => MnemonicType::Words15,
            18 => MnemonicType::Words18,
            21 => MnemonicType::Words21,
            24 => MnemonicType::Words24,
            _ => return Err(block_error::Error::new(format!("Invalid word count provided: {:?}. Valid optiuons are 12, 15, 18, 21, 24", word_count)))
        };

        let mnemonic = String::from(Mnemonic::new(words, Language::English).phrase());

        let wallet = MnemonicBuilder::<English>::default()
            .phrase(&*mnemonic)
            .word_count(24)
            .derivation_path(path)?
            .build()?;

        let private_key = Some(format!("0x{:02X?}", wallet.signer().to_bytes()).replace(", ", "").replace("[", "").replace("]", ""));
        let public_key = Some(format!("0x{:02X?}", wallet.signer().verifying_key().to_bytes()).replace(", ", "").replace("[", "").replace("]", ""));
        let address = Some(format!("0x{:02X?}", wallet.address().as_bytes()).replace(", ", "").replace("[", "").replace("]", ""));

        Ok(Self {
            mnemonic: Some(mnemonic),
            private_key: private_key,
            public_key: public_key,
            address: address,
            path: Some(path.to_string()),
            ..Default::default()
        })
    }

    pub fn from_mnemonic(mnemonic: &str, path: &str) -> Result<Self, block_error::Error> {
        let wallet = MnemonicBuilder::<English>::default()
            .phrase(mnemonic)
            .word_count(24)
            .derivation_path(path)?
            .build()?;

        let private_key = Some(format!("0x{:02X?}", wallet.signer().to_bytes()).replace(", ", "").replace("[", "").replace("]", ""));
        let public_key = Some(format!("0x{:02X?}", wallet.signer().verifying_key().to_bytes()).replace(", ", "").replace("[", "").replace("]", ""));
        let address = Some(format!("0x{:02X?}", wallet.address().as_bytes()).replace(", ", "").replace("[", "").replace("]", ""));

        Ok(Self {
            mnemonic: Some(mnemonic.to_string()),
            private_key: private_key,
            public_key: public_key,
            address: address,
            path: Some(path.to_string()),
            ..Default::default()
        })
    }

    pub fn from_extended_private_key(extended_private_key: &str, path: &Option<String>) -> Result<Self, block_error::Error> {
        Ok(Self {
            address: Some(String::from("0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5")),
            ..Default::default()
        })
    }

    pub fn from_private_key(private_key: &str) -> Result<Self, block_error::Error> {
        let mut private_key = private_key.to_string();

        if private_key.starts_with("0x") || private_key.starts_with("0X") {
            private_key = private_key.replace("0x", "");
            private_key = private_key.replace("0X", "");
        }

        let wallet = match private_key.parse::<LocalWallet>() {
            Ok(wallet) => wallet,
            Err(_) => return Err(block_error::Error::new(format!("Invalid private key provided: {:?}", private_key)))
        };

        let private_key = Some(format!("0x{:02X?}", wallet.signer().to_bytes()).replace(", ", "").replace("[", "").replace("]", ""));
        let public_key = Some(format!("0x{:02X?}", wallet.signer().verifying_key().to_bytes()).replace(", ", "").replace("[", "").replace("]", ""));
        let address = Some(format!("0x{:02X?}", wallet.address().as_bytes()).replace(", ", "").replace("[", "").replace("]", ""));

        Ok(Self {
            private_key: private_key,
            public_key: public_key,
            address: address,
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
        //self.address = Some(String::from("0x28C6c06298d514Db089934071355E5743bf21d60"));
        self.address = Some(String::from("0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5"));
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

        let mut eth_transactions: Vec<EthTransaction> = match serde_json::from_str(&json["result"].to_string()) {
            Ok(eth_transactions) => eth_transactions,
            Err(e) => panic!("Error parsing eth_transactions: {}", e)
        };

        if eth_transactions.len() > 0 {
            *self.last_block.lock().unwrap() = match &eth_transactions[eth_transactions.len() - 1].blockNumber {
                Some(block_number) => block_number.parse::<i64>().expect("ERROR: Parsing block_number failed."),
                None => 0
            };
        } else {
            *self.last_block.lock().unwrap() = 0;
        }

        let address = orig_address.to_uppercase();

        for eth_transaction in eth_transactions.clone() {
            let symbol = match &eth_transaction.tokenSymbol {
                Some(symbol) => symbol,
                None => continue
            };

            if eth_transaction.value == Some("0".to_string()) {
                eth_transactions.remove(eth_transactions.iter().position(|x| x == &eth_transaction).unwrap());
                continue;
            }

            if tokens.contains_key(symbol) {
                let decimals = match &eth_transaction.tokenDecimal {
                    Some(decimals) => decimals.parse::<i32>().expect("ERROR: Parsing decimal failed.") + 1,
                    None => continue
                };
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
                        
                        let mut current_balance: f64 = value.parse::<f64>().expect("ERROR: Parsing value failed.");
                        current_balance = current_balance / CurrencyPairs::get_exponent(decimals);
                        balance = balance + current_balance;
                        self.erc20_balances.lock().unwrap().insert(eth_transaction.tokenSymbol.clone().unwrap(), balance);
                    } else if from == address {
                        let mut balance = self.erc20_balances.lock().unwrap()[&eth_transaction.tokenSymbol.clone().unwrap()];
                        let value = match eth_transaction.value.clone() {
                            Some(value) => value,
                            None => continue
                        };

                        let mut current_balance: f64 = value.parse::<f64>().expect("ERROR: Parsing value failed.");
                        current_balance = current_balance / CurrencyPairs::get_exponent(decimals);
                        balance = balance - current_balance;
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
                        
                        let mut current_balance: f64 = value.parse::<f64>().expect("ERROR: Parsing value failed.");
                        current_balance = current_balance / CurrencyPairs::get_exponent(decimals);
                        balance = balance + current_balance;
                        self.erc20_balances.lock().unwrap().insert(eth_transaction.tokenSymbol.clone().unwrap(), balance);
                    } else if from == address {
                        let mut balance = self.erc20_balances.lock().unwrap()[&eth_transaction.tokenSymbol.clone().unwrap()];
                        let value = match eth_transaction.value.clone() {
                            Some(value) => value,
                            None => continue
                        };
                        
                        let mut current_balance: f64 = value.parse::<f64>().expect("ERROR: Parsing value failed.");
                        current_balance = current_balance / CurrencyPairs::get_exponent(decimals);
                        balance = balance - current_balance;
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

    pub async fn ether_transaction(&self, receiver: &str, amount: u64) -> Result<(), block_error::Error> {
        let provider = match Provider::<Http>::try_from("http://127.0.0.1:8545") { //https://eth.llamarpc.com
            Ok(provider) => provider,
            Err(e) => return Err(block_error::Error::new(e.to_string()))
        };

        let private_key = match &self.private_key {
            Some(private_key) => private_key.to_lowercase().replace("0x", ""),
            None => return Err(block_error::Error::new("ERROR: private_key is not set!".to_string()))
        };

        let wallet = private_key.parse::<LocalWallet>()?;
        let tx;

        if !receiver.to_lowercase().contains(".eth")  {
            let rec = receiver.to_lowercase().replace("0x", "").parse::<H160>()?;
            tx = TransactionRequest::new().to(rec).value(amount).from(wallet.address());
        } else {
            tx = TransactionRequest::new().to(receiver).value(amount).from(wallet.address());
        }
        
        let balance_before = provider.get_balance(wallet.address(), None).await?;
        let nonce1 = provider.get_transaction_count(wallet.address(), None).await?;
        
        let tx = match provider.send_transaction(tx, None).await {
            Ok(tx) => tx,
            Err(e) => return Err(block_error::Error::new(e.to_string()))
        };

        let tx = tx.await?;

        println!("{}", serde_json::to_string(&tx)?);
        Ok(())
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
            },/*
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
            },*/
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