use colored::*;
use core::{fmt, fmt::Display};
use serde::Serialize;
use serde_json::Value;
use fast_qr::convert::{image::ImageBuilder, Builder, Shape};
use fast_qr::qr::QRBuilder;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use alloy::network::TransactionBuilder;
use alloy::primitives::{Address, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::TransactionRequest;
use alloy::signers::k256::ecdsa::SigningKey;
use alloy::signers::local::coins_bip39::English;
use alloy::signers::local::{MnemonicBuilder, PrivateKeySigner};

use bip39::Mnemonic;
use rand::RngCore;

use crate::configuration::*;
use crate::currencies::transactions::*;
use crate::currencies::tokens::*;
use crate::currencies::currency_pairs::*;

const DEFAULT_ETH_PATH: &str = "m/44'/60'/0'/0/0";

pub fn generate_eth_basic_wallet() -> Option<EthereumWallet> {
    match EthereumWallet::new() {
        Ok(eth_wallet) => return Some(eth_wallet),
        Err(_) => {
            crate::configuration::logging::error("ethereum wallet generation failed");
            return None
        }
    };
}

pub fn generate_eth_hd_wallet() -> Option<EthereumWallet> {
    match EthereumWallet::new_hd(
        24,
        DEFAULT_ETH_PATH,
    ) {
        Ok(eth_wallet) => return Some(eth_wallet),
        Err(_) => {
            crate::configuration::logging::error("ethereum HD wallet generation failed");
            return None
        }
    };
}

pub fn generate_from_mnemonic(mnemonic: &str, mut path: &str) -> Option<EthereumWallet> {
    if path.is_empty() {
        path = DEFAULT_ETH_PATH;
    }

    match EthereumWallet::from_mnemonic(mnemonic, path, "") {
        Ok(eth_wallet) => return Some(eth_wallet),
        Err(_) => {
            crate::configuration::logging::error("ethereum wallet from mnemonic failed");
            return None
        }
    };
}

pub fn generate_from_private_key(private_key: &str) -> Option<EthereumWallet> {
    match EthereumWallet::from_private_key(private_key) {
        Ok(eth_wallet) => return Some(eth_wallet),
        Err(_) => {
            crate::configuration::logging::error("ethereum wallet from private key failed");
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
    pub history: Arc<Mutex<Vec<crate::currencies::eth_chain::EthHistoryItem>>>,
    pub last_block: Arc<Mutex<i64>>,
    
}

impl EthereumWallet {
    pub fn new() -> Result<Self, block_error::Error> {
        Self::new_hd(12, DEFAULT_ETH_PATH)
    }

    pub fn new_hd(word_count: u8, path: &str) -> Result<Self, block_error::Error> {
        let entropy_len = match word_count {
            12 => 16,
            15 => 20,
            18 => 24,
            21 => 28,
            24 => 32,
            _ => return Err(block_error::Error::new(format!("Invalid word count provided: {:?}. Valid options are 12, 15, 18, 21, 24", word_count)))
        };
        let mut entropy = vec![0u8; entropy_len];
        rand::thread_rng().fill_bytes(&mut entropy);
        let mnemonic = Mnemonic::from_entropy(&entropy)
            .map_err(|e| block_error::Error::new(format!("Mnemonic generation failed: {:?}", e)))?;
        Self::from_mnemonic(&mnemonic.to_string(), path, "")
    }

    pub fn from_mnemonic(mnemonic: &str, path: &str, passphrase: &str) -> Result<Self, block_error::Error> {
        let mut builder = MnemonicBuilder::<English>::default().phrase(mnemonic);
        builder = builder
            .derivation_path(path)
            .map_err(|e| block_error::Error::new(format!("Invalid derivation path {:?}: {:?}", path, e)))?;
        if !passphrase.is_empty() {
            builder = builder.password(passphrase);
        }
        let signer = builder
            .build()
            .map_err(|e| block_error::Error::new(format!("Alloy mnemonic wallet failed: {:?}", e)))?;

        let mut wallet = wallet_from_signer(signer, Some(mnemonic.to_string()), Some(path.to_string()));
        if !passphrase.is_empty() {
            wallet.password = Some(passphrase.to_string());
        }
        Ok(wallet)
    }

    pub fn from_private_key(private_key: &str) -> Result<Self, block_error::Error> {
        let mut key = private_key.trim().to_string();
        if key.starts_with("0x") || key.starts_with("0X") {
            key = key[2..].to_string();
        }

        let signer = PrivateKeySigner::from_str(&key)
            .or_else(|_| PrivateKeySigner::from_str(&format!("0x{}", key)))
            .map_err(|e| block_error::Error::new(format!("Invalid private key provided: {:?}", e)))?;

        Ok(wallet_from_signer(signer, None, None))
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
            Some(address) => address.clone(),
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
            Err(e) => {
                tracing::warn!("etherscan token history unavailable: {e}");
                return;
            }
        };

        if eth_transactions.len() > 0 {
            *self.last_block.lock().unwrap() = match &eth_transactions[eth_transactions.len() - 1].blockNumber {
                Some(block_number) => block_number.parse::<i64>().unwrap_or(0),
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
                if let Some(index) = eth_transactions.iter().position(|x| x == &eth_transaction) {
                    eth_transactions.remove(index);
                }
                continue;
            }

            if tokens.contains_key(symbol) {
                let decimals = match &eth_transaction.tokenDecimal {
                    Some(decimals) => decimals.parse::<i32>().unwrap_or(0) + 1,
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
                        
                        let mut current_balance: f64 = match value.parse::<f64>() {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        current_balance = current_balance / CurrencyPairs::get_exponent(decimals);
                        balance = balance + current_balance;
                        self.erc20_balances.lock().unwrap().insert(eth_transaction.tokenSymbol.clone().unwrap(), balance);
                    } else if from == address {
                        let mut balance = self.erc20_balances.lock().unwrap()[&eth_transaction.tokenSymbol.clone().unwrap()];
                        let value = match eth_transaction.value.clone() {
                            Some(value) => value,
                            None => continue
                        };

                        let mut current_balance: f64 = match value.parse::<f64>() {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
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
                        
                        let mut current_balance: f64 = match value.parse::<f64>() {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        current_balance = current_balance / CurrencyPairs::get_exponent(decimals);
                        balance = balance + current_balance;
                        self.erc20_balances.lock().unwrap().insert(eth_transaction.tokenSymbol.clone().unwrap(), balance);
                    } else if from == address {
                        let mut balance = self.erc20_balances.lock().unwrap()[&eth_transaction.tokenSymbol.clone().unwrap()];
                        let value = match eth_transaction.value.clone() {
                            Some(value) => value,
                            None => continue
                        };
                        
                        let mut current_balance: f64 = match value.parse::<f64>() {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
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

    pub fn wipe_secrets(&mut self) {
        crate::configuration::secrets::wipe_optional_string(&mut self.mnemonic);
        crate::configuration::secrets::wipe_optional_string(&mut self.password);
        crate::configuration::secrets::wipe_optional_string(&mut self.private_key);
        crate::configuration::secrets::wipe_optional_string(&mut self.public_key);
    }

    pub fn generate_qr_address(&self) -> Result<gdk4::Texture, block_error::Error> {
        let address = match &self.address {
            Some(addr) => addr,
            None => ""
        };

        if address.is_empty() {
            return Err(block_error::Error::new("no receive address for QR".to_string()));
        }
        let qrcode = QRBuilder::new(address.to_string())
            .build()
            .map_err(|e| block_error::Error::new(format!("QR encode failed: {e:?}")))?;

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
        let private_key = match &self.private_key {
            Some(private_key) => private_key.to_lowercase().replace("0x", ""),
            None => return Err(block_error::Error::new("ERROR: private_key is not set!".to_string()))
        };

        if receiver.to_lowercase().contains(".eth") {
            return Err(block_error::Error::new("ENS names are not supported yet".to_string()));
        }

        let signer = PrivateKeySigner::from_str(&private_key)
            .or_else(|_| PrivateKeySigner::from_str(&format!("0x{}", private_key)))
            .map_err(|e| block_error::Error::new(format!("Invalid private key: {:?}", e)))?;

        let to = Address::from_str(receiver)
            .map_err(|e| block_error::Error::new(format!("Invalid receiver address: {:?}", e)))?;

        let url = "http://127.0.0.1:8545"
            .parse()
            .map_err(|e| block_error::Error::new(format!("Invalid ETH RPC URL: {:?}", e)))?;
        let provider = ProviderBuilder::new().wallet(signer).connect_http(url);

        let tx = TransactionRequest::default()
            .with_to(to)
            .with_value(U256::from(amount));

        let pending = provider.send_transaction(tx).await.map_err(|e| {
            block_error::Error::new(format!("ETH send failed: {:?}", e))
        })?;
        let receipt = pending.get_receipt().await.map_err(|e| {
            block_error::Error::new(format!("ETH receipt failed: {:?}", e))
        })?;

        tracing::info!("ethereum transaction submitted");
        let _ = receipt;
        Ok(())
    }
}

fn wallet_from_signer(signer: PrivateKeySigner, mnemonic: Option<String>, path: Option<String>) -> EthereumWallet {
    let private_key = Some(format!("0x{}", hex::encode(signer.to_bytes())));
    let public_key = signing_key_to_public_hex(signer.credential());
    let address = Some(format!("{}", signer.address()));

    EthereumWallet {
        mnemonic,
        private_key,
        public_key,
        address,
        path,
        ..Default::default()
    }
}

fn signing_key_to_public_hex(key: &SigningKey) -> Option<String> {
    let point = key.verifying_key().to_encoded_point(false);
    Some(format!("0x{}", hex::encode(point.as_bytes())))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_from_known_mnemonic() {
        let wallet = generate_from_mnemonic(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            "",
        )
        .unwrap();
        let address = wallet.address.unwrap();
        assert!(address.starts_with("0x"));
        assert_eq!(address.len(), 42);
        assert!(wallet.private_key.is_some());
        assert!(wallet.mnemonic.is_some());
    }

    #[test]
    fn generate_from_private_key_roundtrips_address() {
        let generated = generate_eth_hd_wallet().unwrap();
        let key = generated.private_key.clone().unwrap();
        let wallet = generate_from_private_key(&key).unwrap();
        assert_eq!(wallet.address, generated.address);
        assert!(wallet.public_key.is_some());
    }

    #[test]
    fn wipe_secrets_clears_key_material_and_keeps_address() {
        let mut wallet = EthereumWallet::from_mnemonic(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            "m/44'/60'/0'/0/0",
            "",
        )
        .unwrap();
        let address = wallet.address.clone();
        assert!(wallet.mnemonic.is_some());
        assert!(wallet.private_key.is_some());
        wallet.wipe_secrets();
        assert!(wallet.mnemonic.is_none());
        assert!(wallet.private_key.is_none());
        assert!(wallet.password.is_none());
        assert!(wallet.public_key.is_none());
        assert_eq!(wallet.address, address);
    }

    #[test]
    fn display_omits_mnemonic_and_private_key() {
        let wallet = EthereumWallet::from_mnemonic(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            "m/44'/60'/0'/0/0",
            "",
        )
        .unwrap();
        let key = wallet.private_key.clone().unwrap();
        let rendered = format!("{wallet}");
        assert!(!rendered.contains("abandon"));
        assert!(!rendered.contains(&key));
        assert!(rendered.contains("0x"));
    }
}
