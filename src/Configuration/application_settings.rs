use std::{io, thread, fs};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use glib::{clone, ControlFlow};

use crate::currencies::eth;
use crate::currencies::eth::EthereumWallet;
use crate::currencies::btc;
use crate::currencies::btc::BitcoinWallet;
use crate::currencies::ltc;
use crate::currencies::ltc::LitecoinWallet;
use crate::currencies::sol;
use crate::currencies::sol::SolanaWallet;
use crate::currencies::tokens::*;
use crate::configuration::block_error;
use crate::configuration::seed;
use crate::configuration::wallet_store::{
    BtcRecord, CustomTokenRecord, EthRecord, LtcRecord, PayloadV1, SolRecord, StoreSession, StoreSettings,
};

#[derive(Clone)]
pub struct ApplicationSettings {
    pub config_path         : PathBuf,
    pub error_path          : PathBuf,
    pub backup_path         : PathBuf,
    pub store_session       : Option<StoreSession>,
    pub mnemonic            : Option<String>,
    pub seed_passphrase     : Option<String>,
    pub btc_wallets         : Vec<BitcoinWallet>,
    pub eth_wallets         : Vec<EthereumWallet>,
    pub sol_wallets         : Vec<SolanaWallet>,
    pub ltc_wallets         : Vec<LitecoinWallet>,
    pub tokens              : Tokens,
    pub default_currency    : Token,
    pub starred             : HashMap<String, Token>,
    pub logged_in           : bool,
    pub infura_key          : String,
    pub etherscan_key       : String,
    pub eth_node            : String,
    pub btc_node            : String,
    pub sol_node            : String,
    pub ltc_node            : String,
    pub btc_network         : String,
    pub eth_network         : String,
    pub sol_network         : String,
    pub ltc_network         : String,
    pub custom_tokens       : Vec<CustomTokenRecord>,
    pub lock_timeout_secs   : u32,
    pub show_prices         : bool,
    pub fiat                : String,
    pub btc_units           : String,
    pub sync_epoch          : Arc<AtomicU64>,
}

impl std::fmt::Debug for ApplicationSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApplicationSettings")
            .field("config_path", &self.config_path)
            .field("logged_in", &self.logged_in)
            .field("unlocked", &self.is_unlocked())
            .field("btc_wallets", &self.btc_wallets.len())
            .field("eth_wallets", &self.eth_wallets.len())
            .field("sol_wallets", &self.sol_wallets.len())
            .field("ltc_wallets", &self.ltc_wallets.len())
            .finish_non_exhaustive()
    }
}

impl ApplicationSettings {
    pub fn new(tokens: Tokens) -> Self {
        let cpath = ApplicationSettings::find_config_path().unwrap_or_default();
        let epath = ApplicationSettings::find_error_path().unwrap_or_default();
        let bpath = ApplicationSettings::find_wallet_backup_path().unwrap_or_default();

        let store_session = None;

        let bitcoin_wallets = vec![];
        let ethereum_wallets = vec![];
        let solana_wallets = vec![];
        let litecoin_wallets = vec![];

        if !epath.as_os_str().is_empty() && !std::path::Path::new(&epath).exists() {
            if let Err(why) = File::create(&epath) {
                tracing::warn!(path = %epath.display(), %why, "could not create log file");
            }
        }

        let mut starred  = HashMap::new();
        let mut i_key    = String::new();
        let mut e_key    = String::new();

        if !std::path::Path::new(&cpath).exists() {
            for (key, value) in tokens.eth_tokens.clone() {
                if tokens.eth_tokens[&key].symbol == "BTC" {
                    starred.insert(key.clone(), value.clone());
                } else if tokens.eth_tokens[&key].symbol == "ETH" {
                    starred.insert(key.clone(), value.clone());
                } else if tokens.eth_tokens[&key].symbol == "MATIC" {
                    starred.insert(key.clone(), value.clone());
                } else if tokens.eth_tokens[&key].symbol == "WBTC" {
                    starred.insert(key.clone(), value.clone());
                } else if tokens.eth_tokens[&key].symbol == "UNI" {
                    starred.insert(key.clone(), value.clone());
                } else if tokens.eth_tokens[&key].symbol == "BNB" {
                    starred.insert(key.clone(), value.clone());
                } else if tokens.eth_tokens[&key].symbol == "SHIB" {
                    starred.insert(key.clone(), value.clone());
                } else if tokens.eth_tokens[&key].symbol == "TRON" {
                    starred.insert(key.clone(), value.clone());
                } else if tokens.eth_tokens[&key].symbol == "LINK" {
                    starred.insert(key.clone(), value.clone());
                } else if tokens.eth_tokens[&key].symbol == "QNT" {
                    starred.insert(key.clone(), value.clone());
                } else if tokens.eth_tokens[&key].symbol == "APE" {
                    starred.insert(key.clone(), value.clone());
                } else if tokens.eth_tokens[&key].symbol == "FTM" {
                    starred.insert(key.clone(), value.clone());
                } else if tokens.eth_tokens[&key].symbol == "GRT" {
                    starred.insert(key.clone(), value.clone());
                } else if tokens.eth_tokens[&key].symbol == "SAND" {
                    starred.insert(key.clone(), value.clone());
                } else if tokens.eth_tokens[&key].symbol == "MANA" {
                    starred.insert(key.clone(), value.clone());
                } else if tokens.eth_tokens[&key].symbol == "AXS" {
                    starred.insert(key.clone(), value.clone());
                } else if tokens.eth_tokens[&key].symbol == "CHZ" {
                    starred.insert(key.clone(), value.clone());
                } else if tokens.eth_tokens[&key].symbol == "CRV" {
                    starred.insert(key.clone(), value.clone());
                } else if tokens.eth_tokens[&key].symbol == "SOL" {
                    starred.insert(key.clone(), value.clone());
                } else if tokens.eth_tokens[&key].symbol == "LTC" {
                    starred.insert(key.clone(), value.clone());
                }
            }

            let ypath = match ApplicationSettings::find_network_config_path() {
                Ok(path) => path,
                Err(why) => {
                    tracing::warn!(%why, "could not resolve network config path");
                    PathBuf::new()
                }
            };

            if !ypath.as_os_str().is_empty() && !ypath.exists() {
                match File::create(&ypath) {
                    Ok(_) => {
                        let data = "INFURA_KEY=\nETHERSCAN_KEY=\n";
                        if let Err(why) = fs::write(&ypath, data) {
                            tracing::warn!(path = %ypath.display(), %why, "could not write network config");
                        }
                    },
                    Err(why) => tracing::warn!(path = %ypath.display(), %why, "could not create network config"),
                };
            } else if ypath.exists() {
                let printable_path = ypath.clone();
                let content = match fs::read_to_string(ypath) {
                    Ok(content) => content,
                    Err(why) => {
                        tracing::warn!(path = %printable_path.display(), %why, "could not read network config");
                        String::new()
                    }
                };

                let contents: Vec<&str> = content.split("\n").collect();
    
                for i in 0..contents.len() {
                    if let Some(value) = contents[i].strip_prefix("INFURA_KEY=") {
                        i_key = value.to_string();
                    } else if let Some(value) = contents[i].strip_prefix("ETHERSCAN_KEY=") {
                        e_key = value.to_string();
                    }
                }
            }
        }

        let default = Token {
            name    : String::from("USD Coin"),
            symbol  : String::from("USDC"),
            address : String::from("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"),
            logo    : crate::configuration::paths::token_icon_path("USDC"),
            decimals: 6,
            chain   : String::from("eth")
        };

        let mut settings = ApplicationSettings {
            config_path         : cpath,
            error_path          : epath,
            backup_path         : bpath,
            store_session       : store_session,
            mnemonic            : None,
            seed_passphrase     : None,
            btc_wallets         : bitcoin_wallets,
            eth_wallets         : ethereum_wallets,
            sol_wallets         : solana_wallets,
            ltc_wallets         : litecoin_wallets,
            tokens              : tokens,
            default_currency    : default,
            starred             : starred,
            logged_in           : false,
            infura_key          : i_key,
            etherscan_key       : e_key,
            eth_node            : String::new(),
            btc_node            : String::new(),
            sol_node            : String::new(),
            ltc_node            : String::new(),
            btc_network         : String::from("bitcoin"),
            eth_network         : String::from("mainnet"),
            sol_network         : String::from("mainnet"),
            ltc_network         : String::from("litecoin"),
            custom_tokens       : Vec::new(),
            lock_timeout_secs   : 120,
            show_prices         : false,
            fiat                : String::from("usd"),
            btc_units           : String::from("btc"),
            sync_epoch          : Arc::new(AtomicU64::new(0)),
        };
        crate::currencies::eth_chain::apply_bundled_tokens(
            &mut settings.tokens,
            crate::currencies::eth_chain::parse_network(&settings.eth_network),
        );
        crate::currencies::sol_chain::apply_bundled_tokens(
            &mut settings.tokens,
            crate::currencies::sol_chain::parse_network(&settings.sol_network),
        );
        settings
    }

    pub fn generate_btc_wallet(wallet_name: String) -> Result<BitcoinWallet, block_error::Error> {
        let mut bitcoin_wallet = btc::BitcoinWallet::new()?;
        if wallet_name.is_empty() {
            bitcoin_wallet.set_wallet_name(String::from("btc_wallet"));
        } else {
            bitcoin_wallet.set_wallet_name(wallet_name);
        }
        Ok(bitcoin_wallet)
    }

    pub fn generate_eth_wallet(wallet_name: String) -> Result<EthereumWallet, block_error::Error> {
        let mut ethereum_wallet = eth::generate_eth_hd_wallet().ok_or_else(|| {
            block_error::Error::new("ethereum wallet generation failed".to_string())
        })?;
        if wallet_name.is_empty() {
            ethereum_wallet.set_wallet_name(String::from("eth_wallet"));
        } else {
            ethereum_wallet.set_wallet_name(wallet_name);
        }
        Ok(ethereum_wallet)
    }

    pub fn generate_sol_wallet(wallet_name: String) -> Result<SolanaWallet, block_error::Error> {
        let mut solana_wallet = sol::generate_sol_hd_wallet().ok_or_else(|| {
            block_error::Error::new("solana wallet generation failed".to_string())
        })?;
        if wallet_name.is_empty() {
            solana_wallet.set_wallet_name(String::from("sol_wallet"));
        } else {
            solana_wallet.set_wallet_name(wallet_name);
        }
        Ok(solana_wallet)
    }

    pub fn generate_ltc_wallet(wallet_name: String) -> Result<LitecoinWallet, block_error::Error> {
        let mut litecoin_wallet = ltc::generate_ltc_hd_wallet().ok_or_else(|| {
            block_error::Error::new("litecoin wallet generation failed".to_string())
        })?;
        if wallet_name.is_empty() {
            litecoin_wallet.set_wallet_name(String::from("ltc_wallet"));
        } else {
            litecoin_wallet.set_wallet_name(wallet_name);
        }
        Ok(litecoin_wallet)
    }

    pub fn apply_test_networks(&mut self, test: bool) {
        if test {
            let _ = self.apply_btc_network("testnet");
            self.apply_eth_network("sepolia");
            self.apply_sol_network("devnet");
            self.apply_ltc_network("testnet");
        } else {
            let _ = self.apply_btc_network("bitcoin");
            self.apply_eth_network("mainnet");
            self.apply_sol_network("mainnet");
            self.apply_ltc_network("litecoin");
        }
    }

    pub fn is_test_mode(&self) -> bool {
        self.btc_network.eq_ignore_ascii_case("testnet")
            && self.eth_network.eq_ignore_ascii_case("sepolia")
            && self.sol_network.eq_ignore_ascii_case("devnet")
            && self.ltc_network.eq_ignore_ascii_case("testnet")
    }

    pub fn find_config_path() -> io::Result<PathBuf> {
        crate::configuration::paths::wallet_store_path()
    }

    pub fn find_error_path() -> io::Result<PathBuf> {
        crate::configuration::paths::log_path()
    }

    pub fn find_images_path() -> io::Result<PathBuf> {
        crate::configuration::paths::images_path()
    }

    pub fn find_wallet_backup_path() -> io::Result<PathBuf> {
        crate::configuration::paths::backup_dir()
    }

    pub fn find_network_config_path() -> io::Result<PathBuf> {
        crate::configuration::paths::network_config_path()
    }

    pub fn find_currency_details_path() -> io::Result<PathBuf> {
        crate::configuration::paths::currency_details_path()
    }

    pub fn unlock_store(&mut self, password: &str) -> Result<(), block_error::Error> {
        let (payload, session) = StoreSession::unlock(&self.config_path, password)?;
        if let Err(err) = self.apply_payload(payload) {
            self.lock_store();
            return Err(err);
        }
        self.store_session = Some(session);
        self.logged_in = true;
        Ok(())
    }

    pub fn apply_btc_network(&mut self, name: &str) -> Result<(), block_error::Error> {
        let network = crate::currencies::btc_chain::parse_network(name);
        self.btc_network = crate::currencies::btc_chain::network_name(network).to_string();
        let Some(phrase) = self.mnemonic.clone() else {
            return Ok(());
        };
        let passphrase = self.seed_passphrase.clone().unwrap_or_default();
        let extras: Vec<BitcoinWallet> = self
            .btc_wallets
            .iter()
            .filter(|wallet| wallet.mnemonic.as_deref() != Some(phrase.as_str()))
            .cloned()
            .collect();
        let primary = seed::bitcoin_from_seed_on(&phrase, &passphrase, "Bitcoin", network)?;
        self.btc_wallets = vec![primary];
        self.btc_wallets.extend(extras);
        Ok(())
    }

    pub fn apply_eth_network(&mut self, name: &str) {
        let network = crate::currencies::eth_chain::parse_network(name);
        self.eth_network = crate::currencies::eth_chain::network_name(network).to_string();
        // Drop the previous network's bundled tokens (but keep anything the user added
        // themselves) so switching networks can't leave a stale contract address sitting in
        // the registry under a reused symbol key, e.g. "eth:USDC" pointing at the wrong chain.
        let custom_eth_symbols: std::collections::HashSet<String> = self
            .custom_tokens
            .iter()
            .filter(|record| record.chain_or_default() == "eth")
            .map(|record| record.symbol.clone())
            .collect();
        self.tokens.eth_tokens.retain(|key, token| {
            !(token.chain == "eth" && key.starts_with("eth:") && !custom_eth_symbols.contains(&token.symbol))
        });
        crate::currencies::eth_chain::apply_bundled_tokens(&mut self.tokens, network);
        self.apply_custom_tokens();
        for wallet in &mut self.eth_wallets {
            wallet.erc20_balances.lock().unwrap().clear();
        }
    }

    pub fn apply_sol_network(&mut self, name: &str) {
        let network = crate::currencies::sol_chain::parse_network(name);
        self.sol_network = crate::currencies::sol_chain::network_name(network).to_string();
        crate::currencies::sol_chain::apply_bundled_tokens(&mut self.tokens, network);
        self.apply_custom_tokens();
    }

    pub fn apply_ltc_network(&mut self, name: &str) {
        // LTC has no per-network bundled token list (there's exactly one static "LTC" entry in
        // the registry, mainnet and testnet alike), so unlike apply_eth_network/apply_sol_network
        // there's no token re-bundling to do here.
        let network = crate::currencies::ltc_chain::parse_network(name);
        self.ltc_network = crate::currencies::ltc_chain::network_name(network).to_string();
    }

    pub fn apply_custom_tokens(&mut self) {
        for record in &self.custom_tokens {
            let chain = record.chain_or_default();
            self.tokens.eth_tokens.insert(
                format!("{chain}:{}", record.symbol),
                Token {
                    name: record.name.clone(),
                    symbol: record.symbol.clone(),
                    address: record.address.clone(),
                    logo: crate::configuration::paths::token_icon_path(&record.symbol),
                    decimals: record.decimals,
                    chain,
                },
            );
        }
    }

    pub fn add_custom_token(&mut self, record: CustomTokenRecord) {
        let chain = record.chain_or_default();
        self.tokens.eth_tokens.insert(
            format!("{chain}:{}", record.symbol),
            Token {
                name: record.name.clone(),
                symbol: record.symbol.clone(),
                address: record.address.clone(),
                logo: crate::configuration::paths::token_icon_path(&record.symbol),
                decimals: record.decimals,
                chain,
            },
        );
        if !self.custom_tokens.iter().any(|existing| {
            existing.address.eq_ignore_ascii_case(&record.address)
        }) {
            self.custom_tokens.push(record);
        }
    }

    pub fn is_unlocked(&self) -> bool {
        self.logged_in && self.store_session.is_some() && self.mnemonic.is_some()
    }

    pub fn verify_password(&self, password: &str) -> bool {
        if password.is_empty() || !self.config_path.exists() {
            return false;
        }
        StoreSession::unlock(&self.config_path, password).is_ok()
    }

    pub fn secrets_for_address(&self, address: &str) -> (Option<String>, Option<String>) {
        for wallet in &self.btc_wallets {
            if wallet.address.as_deref() == Some(address) {
                return (
                    wallet.mnemonic.clone().or_else(|| self.mnemonic.clone()),
                    wallet.private_key.clone(),
                );
            }
        }
        for wallet in &self.eth_wallets {
            if wallet.address.as_deref() == Some(address) {
                return (
                    wallet.mnemonic.clone().or_else(|| self.mnemonic.clone()),
                    wallet.private_key.clone(),
                );
            }
        }
        for wallet in &self.sol_wallets {
            if wallet.address.as_deref() == Some(address) {
                return (
                    wallet.mnemonic.clone().or_else(|| self.mnemonic.clone()),
                    wallet.private_key.clone(),
                );
            }
        }
        for wallet in &self.ltc_wallets {
            if wallet.address.as_deref() == Some(address) {
                return (
                    wallet.mnemonic.clone().or_else(|| self.mnemonic.clone()),
                    wallet.private_key.clone(),
                );
            }
        }
        (None, None)
    }

    pub fn lock_store(&mut self) {
        if let Some(session) = self.store_session.clone() {
            let _ = session.save(&self.to_payload());
        }
        if let Some(mut session) = self.store_session.take() {
            session.wipe();
        }
        crate::configuration::secrets::wipe_optional_string(&mut self.mnemonic);
        crate::configuration::secrets::wipe_optional_string(&mut self.seed_passphrase);
        for wallet in &mut self.btc_wallets {
            wallet.wipe_secrets();
        }
        for wallet in &mut self.eth_wallets {
            wallet.wipe_secrets();
        }
        for wallet in &mut self.sol_wallets {
            wallet.wipe_secrets();
        }
        for wallet in &mut self.ltc_wallets {
            wallet.wipe_secrets();
        }
        self.btc_wallets.clear();
        self.eth_wallets.clear();
        self.sol_wallets.clear();
        self.ltc_wallets.clear();
        self.logged_in = false;
        self.sync_epoch.fetch_add(1, Ordering::SeqCst);
    }

    pub fn restore_from_mnemonic(&mut self, phrase: &str, passphrase: &str) -> Result<(), block_error::Error> {
        let phrase = seed::parse_mnemonic(phrase)?;
        let network = crate::currencies::btc_chain::parse_network(&self.btc_network);
        let (btc, eth, sol, ltc) = seed::accounts_from_seed_on(&phrase, passphrase, network)?;
        self.mnemonic = Some(phrase);
        self.seed_passphrase = if passphrase.is_empty() {
            None
        } else {
            Some(passphrase.to_string())
        };
        self.btc_wallets = vec![btc];
        self.eth_wallets = vec![eth];
        self.sol_wallets = vec![sol];
        self.ltc_wallets = vec![ltc];
        Ok(())
    }

    pub fn finish_onboarding(
        &mut self,
        phrase: &str,
        passphrase: &str,
        password: &str,
    ) -> Result<(), block_error::Error> {
        crate::configuration::onboarding::validate_password(password, password)
            .map_err(|err| block_error::Error::new(err.as_label().to_string()))?;
        self.restore_from_mnemonic(phrase, passphrase)?;
        self.create_store(password)
    }

    fn ensure_primary_accounts(&mut self) -> Result<(), block_error::Error> {
        if self.mnemonic.is_none() {
            self.mnemonic = Some(seed::generate_mnemonic()?);
        }
        let phrase = self.mnemonic.clone().ok_or_else(|| {
            block_error::Error::new("wallet seed is missing".to_string())
        })?;
        let passphrase = self
            .seed_passphrase
            .clone()
            .unwrap_or_else(|| seed::BTC_PASSPHRASE.to_string());
        let network = crate::currencies::btc_chain::parse_network(&self.btc_network);
        if self.btc_wallets.is_empty() {
            self.btc_wallets.push(seed::bitcoin_from_seed_on(
                &phrase,
                &passphrase,
                "Bitcoin",
                network,
            )?);
        }
        if self.eth_wallets.is_empty() {
            self.eth_wallets.push(seed::ethereum_from_seed(
                &phrase,
                seed::ETH_PATH,
                &passphrase,
                "Ethereum",
            )?);
        }
        if self.sol_wallets.is_empty() {
            self.sol_wallets.push(seed::solana_from_seed(
                &phrase,
                &passphrase,
                "Solana",
            )?);
        }
        if self.ltc_wallets.is_empty() {
            let ltc_network = crate::currencies::ltc_chain::parse_network(&self.ltc_network);
            self.ltc_wallets.push(seed::litecoin_from_seed_on(
                &phrase,
                &passphrase,
                "Litecoin",
                ltc_network,
            )?);
        }
        Ok(())
    }

    fn uses_store_seed(&self, wallet_mnemonic: Option<&str>) -> bool {
        matches!(
            (self.mnemonic.as_deref(), wallet_mnemonic),
            (Some(store), Some(wallet)) if !store.is_empty() && store == wallet
        )
    }

    pub fn create_store(&mut self, password: &str) -> Result<(), block_error::Error> {
        self.ensure_primary_accounts()?;
        let payload = self.to_payload();
        let session = StoreSession::create(&self.config_path, password, &payload)?;
        self.store_session = Some(session);
        self.logged_in = true;
        Ok(())
    }

    pub fn write_config(&mut self) -> Result<bool, block_error::Error> {
        let Some(session) = self.store_session.clone() else {
            crate::configuration::logging::error("cannot save wallet store while locked");
            return Err(block_error::Error::new("wallet store is locked".to_string()));
        };
        session.save(&self.to_payload())?;
        let _ = self.backup_store();
        Ok(true)
    }

    fn to_payload(&self) -> PayloadV1 {
        PayloadV1 {
            schema: crate::configuration::wallet_store::SCHEMA_VERSION,
            mnemonic: self.mnemonic.clone(),
            passphrase: self.seed_passphrase.clone(),
            settings: StoreSettings {
                starred: self.starred.values().map(|token| token.symbol.clone()).collect(),
                infura_key: self.infura_key.clone(),
                etherscan_key: self.etherscan_key.clone(),
                btc_node: self.btc_node.clone(),
                eth_node: self.eth_node.clone(),
                sol_node: self.sol_node.clone(),
                ltc_node: self.ltc_node.clone(),
                btc_network: self.btc_network.clone(),
                eth_network: self.eth_network.clone(),
                sol_network: self.sol_network.clone(),
                ltc_network: self.ltc_network.clone(),
                custom_tokens: self.custom_tokens.clone(),
                lock_timeout_secs: self.lock_timeout_secs,
                show_prices: self.show_prices,
                fiat: self.fiat.clone(),
                btc_units: self.btc_units.clone(),
            },
            btc: self.btc_wallets.iter().map(|wallet| {
                let from_seed = self.uses_store_seed(wallet.mnemonic.as_deref());
                BtcRecord {
                    name: wallet.wallet_name.clone().unwrap_or_default(),
                    mnemonic: if from_seed { None } else { wallet.mnemonic.clone() },
                    passphrase: if from_seed { None } else { wallet.password.clone() },
                    private_key_wif: if from_seed { None } else { wallet.private_key.clone() },
                }
            }).collect(),
            eth: self.eth_wallets.iter().map(|wallet| {
                let from_seed = self.uses_store_seed(wallet.mnemonic.as_deref());
                EthRecord {
                    name: wallet.wallet_name.clone().unwrap_or_default(),
                    mnemonic: if from_seed { None } else { wallet.mnemonic.clone() },
                    path: wallet.path.clone().or_else(|| {
                        if from_seed {
                            Some(seed::ETH_PATH.to_string())
                        } else {
                            None
                        }
                    }),
                    private_key: if from_seed { None } else { wallet.private_key.clone() },
                }
            }).collect(),
            sol: self.sol_wallets.iter().map(|wallet| {
                let from_seed = self.uses_store_seed(wallet.mnemonic.as_deref());
                SolRecord {
                    name: wallet.wallet_name.clone().unwrap_or_default(),
                    mnemonic: if from_seed { None } else { wallet.mnemonic.clone() },
                    path: wallet.path.clone().or_else(|| {
                        if from_seed {
                            Some(seed::SOL_PATH.to_string())
                        } else {
                            None
                        }
                    }),
                    private_key: if from_seed { None } else { wallet.private_key.clone() },
                }
            }).collect(),
            ltc: self.ltc_wallets.iter().map(|wallet| {
                let from_seed = self.uses_store_seed(wallet.mnemonic.as_deref());
                LtcRecord {
                    name: wallet.wallet_name.clone().unwrap_or_default(),
                    mnemonic: if from_seed { None } else { wallet.mnemonic.clone() },
                    passphrase: if from_seed { None } else { wallet.password.clone() },
                    private_key_wif: if from_seed { None } else { wallet.private_key.clone() },
                }
            }).collect(),
        }
    }

    fn apply_payload(&mut self, payload: PayloadV1) -> Result<(), block_error::Error> {
        self.starred.clear();
        for symbol in payload.settings.starred {
            if let Some((key, token)) = self.tokens.eth_tokens.iter().find(|(_, token)| token.symbol == symbol) {
                self.starred.insert(key.clone(), token.clone());
            }
        }
        self.infura_key = payload.settings.infura_key;
        self.etherscan_key = payload.settings.etherscan_key;
        self.btc_node = payload.settings.btc_node;
        if !payload.settings.eth_node.is_empty() {
            self.eth_node = payload.settings.eth_node;
        }
        if !payload.settings.sol_node.is_empty() {
            self.sol_node = payload.settings.sol_node;
        }
        if !payload.settings.ltc_node.is_empty() {
            self.ltc_node = payload.settings.ltc_node;
        }
        if !payload.settings.btc_network.is_empty() {
            self.btc_network = payload.settings.btc_network;
        }
        if !payload.settings.eth_network.is_empty() {
            self.eth_network = payload.settings.eth_network;
        }
        if !payload.settings.sol_network.is_empty() {
            self.sol_network = payload.settings.sol_network;
        }
        if !payload.settings.ltc_network.is_empty() {
            self.ltc_network = payload.settings.ltc_network;
        }
        self.custom_tokens = payload.settings.custom_tokens;
        self.lock_timeout_secs = payload.settings.lock_timeout_secs;
        self.show_prices = payload.settings.show_prices;
        if !payload.settings.fiat.is_empty() {
            self.fiat = payload.settings.fiat;
        }
        if !payload.settings.btc_units.is_empty() {
            self.btc_units = payload.settings.btc_units;
        }
        crate::currencies::eth_chain::apply_bundled_tokens(
            &mut self.tokens,
            crate::currencies::eth_chain::parse_network(&self.eth_network),
        );
        crate::currencies::sol_chain::apply_bundled_tokens(
            &mut self.tokens,
            crate::currencies::sol_chain::parse_network(&self.sol_network),
        );
        self.apply_custom_tokens();

        self.mnemonic = match payload.mnemonic.as_ref().filter(|value| !value.is_empty()) {
            Some(phrase) => Some(seed::parse_mnemonic(phrase)?),
            None => None,
        };
        self.seed_passphrase = payload.passphrase.filter(|value| !value.is_empty());
        let seed_passphrase = self
            .seed_passphrase
            .clone()
            .unwrap_or_else(|| seed::BTC_PASSPHRASE.to_string());

        self.btc_wallets.clear();
        let btc_network = crate::currencies::btc_chain::parse_network(&self.btc_network);
        for record in payload.btc {
            let record_passphrase = record
                .passphrase
                .as_deref()
                .filter(|value| !value.is_empty())
                .unwrap_or(seed_passphrase.as_str());
            let mut wallet = if let Some(mnemonic) = record.mnemonic.as_ref().filter(|value| !value.is_empty()) {
                BitcoinWallet::from_mnemonic_on(mnemonic, record_passphrase, btc_network)?
            } else if let Some(wif) = record.private_key_wif.as_ref().filter(|value| !value.is_empty()) {
                BitcoinWallet::from_private_key(wif)?
            } else if let Some(phrase) = self.mnemonic.clone() {
                seed::bitcoin_from_seed_on(&phrase, record_passphrase, &record.name, btc_network)?
            } else {
                continue;
            };
            if !record.name.is_empty() {
                wallet.set_wallet_name(record.name);
            }
            self.btc_wallets.push(wallet);
        }
        if self.btc_wallets.is_empty() {
            if let Some(phrase) = self.mnemonic.clone() {
                self.btc_wallets.push(seed::bitcoin_from_seed_on(
                    &phrase,
                    &seed_passphrase,
                    "Bitcoin",
                    btc_network,
                )?);
            }
        }

        self.eth_wallets.clear();
        for record in payload.eth {
            let path = record.path.clone().filter(|value| !value.is_empty())
                .unwrap_or_else(|| seed::ETH_PATH.to_string());
            let mut wallet = if let Some(mnemonic) = record.mnemonic.as_ref().filter(|value| !value.is_empty()) {
                let pass = if self.mnemonic.as_deref() == Some(mnemonic.as_str()) {
                    seed_passphrase.as_str()
                } else {
                    ""
                };
                EthereumWallet::from_mnemonic(mnemonic, &path, pass)?
            } else if let Some(key) = record.private_key.as_ref().filter(|value| !value.is_empty()) {
                EthereumWallet::from_private_key(key)?
            } else if let Some(phrase) = self.mnemonic.clone() {
                seed::ethereum_from_seed(&phrase, &path, &seed_passphrase, &record.name)?
            } else {
                continue;
            };
            if !record.name.is_empty() {
                wallet.set_wallet_name(record.name);
            }
            self.eth_wallets.push(wallet);
        }
        if self.eth_wallets.is_empty() {
            if let Some(phrase) = self.mnemonic.clone() {
                self.eth_wallets.push(seed::ethereum_from_seed(
                    &phrase,
                    seed::ETH_PATH,
                    &seed_passphrase,
                    "Ethereum",
                )?);
            }
        }

        self.sol_wallets.clear();
        for record in payload.sol {
            let path = record.path.clone().filter(|value| !value.is_empty())
                .unwrap_or_else(|| seed::SOL_PATH.to_string());
            let mut wallet = if let Some(mnemonic) = record.mnemonic.as_ref().filter(|value| !value.is_empty()) {
                let pass = if self.mnemonic.as_deref() == Some(mnemonic.as_str()) {
                    seed_passphrase.as_str()
                } else {
                    ""
                };
                SolanaWallet::from_mnemonic(mnemonic, &path, pass)?
            } else if let Some(key) = record.private_key.as_ref().filter(|value| !value.is_empty()) {
                SolanaWallet::from_private_key(key)?
            } else if let Some(phrase) = self.mnemonic.clone() {
                seed::solana_from_seed(&phrase, &seed_passphrase, &record.name)?
            } else {
                continue;
            };
            if !record.name.is_empty() {
                wallet.set_wallet_name(record.name);
            }
            self.sol_wallets.push(wallet);
        }
        if self.sol_wallets.is_empty() {
            if let Some(phrase) = self.mnemonic.clone() {
                self.sol_wallets.push(seed::solana_from_seed(
                    &phrase,
                    &seed_passphrase,
                    "Solana",
                )?);
            }
        }

        self.ltc_wallets.clear();
        let ltc_network = crate::currencies::ltc_chain::parse_network(&self.ltc_network);
        for record in payload.ltc {
            let record_passphrase = record
                .passphrase
                .as_deref()
                .filter(|value| !value.is_empty())
                .unwrap_or(seed_passphrase.as_str());
            let mut wallet = if let Some(mnemonic) = record.mnemonic.as_ref().filter(|value| !value.is_empty()) {
                LitecoinWallet::from_mnemonic_on(mnemonic, record_passphrase, ltc_network)?
            } else if let Some(wif) = record.private_key_wif.as_ref().filter(|value| !value.is_empty()) {
                LitecoinWallet::from_private_key(wif)?
            } else if let Some(phrase) = self.mnemonic.clone() {
                seed::litecoin_from_seed_on(&phrase, record_passphrase, &record.name, ltc_network)?
            } else {
                continue;
            };
            if !record.name.is_empty() {
                wallet.set_wallet_name(record.name);
            }
            self.ltc_wallets.push(wallet);
        }
        if self.ltc_wallets.is_empty() {
            if let Some(phrase) = self.mnemonic.clone() {
                self.ltc_wallets.push(seed::litecoin_from_seed_on(
                    &phrase,
                    &seed_passphrase,
                    "Litecoin",
                    ltc_network,
                )?);
            }
        }
        Ok(())
    }

    pub fn backup_store(&self) -> Result<bool, block_error::Error> {
        if !self.config_path.exists() {
            return Ok(false);
        }
        let backup_dir = ApplicationSettings::find_wallet_backup_path()?;
        let name = format!("Config-{}.dic", chrono::Local::now().format("%Y%m%d-%H%M%S"));
        fs::copy(&self.config_path, backup_dir.join(name))?;
        Ok(true)
    }

    pub fn write_error(&self, err: String) {
        crate::configuration::logging::error(&err);
    }

    pub fn write_error_to_path(_pathbuf: &PathBuf, err: String) {
        crate::configuration::logging::error(&err);
    }

    pub fn update_balances(&self) {
        let mut run_before = false;

        for i in 0..self.btc_wallets.len() {
            let btc_balance_arc = Arc::clone(&self.btc_wallets[i].balance);
            let history_arc = Arc::clone(&self.btc_wallets[i].history);
            let mnemonic = self.btc_wallets[i].mnemonic.clone().unwrap_or_default();
            let passphrase = self.btc_wallets[i].password.clone().unwrap_or_default();
            let network = self.btc_network.clone();
            let btc_node = self.btc_node.clone();
            let epoch = Arc::clone(&self.sync_epoch);
            let start_epoch = epoch.load(Ordering::SeqCst);
    
            let (sender, receiver) = crate::configuration::ui_channel::unbounded();
    
            thread::spawn(move || {
                loop {
                    if epoch.load(Ordering::SeqCst) != start_epoch {
                        break;
                    }
                    if run_before == false {
                        thread::sleep(Duration::from_secs(1));
                        run_before = true;
                    }
                    else {
                        thread::sleep(Duration::from_secs(30));
                    }
                    if mnemonic.is_empty() {
                        if sender.send_blocking(String::from("Uninitialized")).is_err() {
                            break;
                        }
                        continue;
                    }
                    let label = match BitcoinWallet::sync_from_seed(
                        &mnemonic,
                        &passphrase,
                        &network,
                        &btc_node,
                    ) {
                        Ok(state) => {
                            *history_arc.lock().unwrap() = state.history.clone();
                            state.balance_display()
                        }
                        Err(_) => String::from("offline"),
                    };
                    if sender.send_blocking(label).is_err() {
                        break;
                    }
                }
            });

            crate::configuration::ui_channel::attach(
                receiver,
                clone!(
                    #[weak]
                    btc_balance_arc,
                    #[upgrade_or]
                    ControlFlow::Break,
                    move |price_text| {
                        let mut btc_balance = btc_balance_arc.lock().unwrap();
                        if price_text != "Uninitialized" {
                            *btc_balance = price_text;
                        }

                        ControlFlow::Continue
                    }
                ),
            );
        }
        for i in 0..self.eth_wallets.len() {
            let eth_balance_arc = Arc::clone(&self.eth_wallets[i].balance);
            let history_arc = Arc::clone(&self.eth_wallets[i].history);
            let erc20_arc = Arc::clone(&self.eth_wallets[i].erc20_balances);
            let address = match &self.eth_wallets[i].address {
                Some(b) => String::from(b),
                None    => String::from("Uninitialized")
            };
            let eth_node = self.eth_node.clone();
            let eth_network = self.eth_network.clone();
            let infura_key = self.infura_key.clone();
            let etherscan_key = self.etherscan_key.clone();
            let tokens: Vec<Token> = self.tokens.eth_tokens.values().cloned().collect();
            let epoch = Arc::clone(&self.sync_epoch);
            let start_epoch = epoch.load(Ordering::SeqCst);

            let (sender, receiver) = crate::configuration::ui_channel::unbounded();
            thread::spawn(move || {
                let mut first = true;
                loop {
                    if epoch.load(Ordering::SeqCst) != start_epoch {
                        break;
                    }
                    if first {
                        thread::sleep(Duration::from_secs(1));
                        first = false;
                    } else {
                        thread::sleep(Duration::from_secs(20));
                    }
                    if address == "Uninitialized" {
                        if sender.send_blocking(String::from("Uninitialized")).is_err() {
                            break;
                        }
                        continue;
                    }
                    let label = match crate::currencies::eth_chain::sync_account(
                        &address,
                        &eth_node,
                        &eth_network,
                        &infura_key,
                        &tokens,
                        &etherscan_key,
                    ) {
                        Ok(state) => {
                            *history_arc.lock().unwrap() = state.history.clone();
                            let mut erc20 = erc20_arc.lock().unwrap();
                            erc20.clear();
                            for (symbol, amount) in &state.erc20 {
                                if let Ok(value) = amount.parse::<f64>() {
                                    erc20.insert(symbol.clone(), value);
                                }
                            }
                            state.balance_display()
                        }
                        Err(_) => String::from("offline"),
                    };
                    if sender.send_blocking(label).is_err() {
                        break;
                    }
                }
            });

            crate::configuration::ui_channel::attach(
                receiver,
                clone!(
                    #[weak]
                    eth_balance_arc,
                    #[upgrade_or]
                    ControlFlow::Break,
                    move |price_text| {
                        let mut eth_balance = eth_balance_arc.lock().unwrap();
                        if price_text != "Uninitialized" {
                            *eth_balance = price_text;
                        }

                        ControlFlow::Continue
                    }
                ),
            );
        }
        for i in 0..self.sol_wallets.len() {
            let sol_balance_arc = Arc::clone(&self.sol_wallets[i].balance);
            let history_arc = Arc::clone(&self.sol_wallets[i].history);
            let spl_arc = Arc::clone(&self.sol_wallets[i].spl_balances);
            let address = match &self.sol_wallets[i].address {
                Some(b) => String::from(b),
                None    => String::from("Uninitialized")
            };
            let sol_node = self.sol_node.clone();
            let sol_network = self.sol_network.clone();
            let tokens: Vec<Token> = self.tokens.eth_tokens.values().cloned().collect();
            let epoch = Arc::clone(&self.sync_epoch);
            let start_epoch = epoch.load(Ordering::SeqCst);

            let (sender, receiver) = crate::configuration::ui_channel::unbounded();
            thread::spawn(move || {
                let mut first = true;
                loop {
                    if epoch.load(Ordering::SeqCst) != start_epoch {
                        break;
                    }
                    if first {
                        thread::sleep(Duration::from_secs(1));
                        first = false;
                    } else {
                        thread::sleep(Duration::from_secs(20));
                    }
                    if address == "Uninitialized" {
                        if sender.send_blocking(String::from("Uninitialized")).is_err() {
                            break;
                        }
                        continue;
                    }
                    let label = match crate::currencies::sol_chain::sync_account(
                        &address,
                        &sol_node,
                        &sol_network,
                        &tokens,
                    ) {
                        Ok(state) => {
                            *history_arc.lock().unwrap() = state.history.clone();
                            let mut spl = spl_arc.lock().unwrap();
                            spl.clear();
                            for (symbol, amount) in &state.spl {
                                if let Ok(value) = amount.parse::<f64>() {
                                    spl.insert(symbol.clone(), value);
                                }
                            }
                            state.balance_display()
                        }
                        Err(_) => String::from("offline"),
                    };
                    if sender.send_blocking(label).is_err() {
                        break;
                    }
                }
            });

            crate::configuration::ui_channel::attach(
                receiver,
                clone!(
                    #[weak]
                    sol_balance_arc,
                    #[upgrade_or]
                    ControlFlow::Break,
                    move |price_text| {
                        let mut sol_balance = sol_balance_arc.lock().unwrap();
                        if price_text != "Uninitialized" {
                            *sol_balance = price_text;
                        }

                        ControlFlow::Continue
                    }
                ),
            );
        }
        for i in 0..self.ltc_wallets.len() {
            let ltc_balance_arc = Arc::clone(&self.ltc_wallets[i].balance);
            let history_arc = Arc::clone(&self.ltc_wallets[i].history);
            let address = match &self.ltc_wallets[i].address {
                Some(b) => String::from(b),
                None    => String::from("Uninitialized")
            };
            let ltc_node = self.ltc_node.clone();
            let ltc_network = self.ltc_network.clone();
            let epoch = Arc::clone(&self.sync_epoch);
            let start_epoch = epoch.load(Ordering::SeqCst);

            let (sender, receiver) = crate::configuration::ui_channel::unbounded();
            thread::spawn(move || {
                let mut first = true;
                loop {
                    if epoch.load(Ordering::SeqCst) != start_epoch {
                        break;
                    }
                    if first {
                        thread::sleep(Duration::from_secs(1));
                        first = false;
                    } else {
                        thread::sleep(Duration::from_secs(30));
                    }
                    if address == "Uninitialized" {
                        if sender.send_blocking(String::from("Uninitialized")).is_err() {
                            break;
                        }
                        continue;
                    }
                    let label = match crate::currencies::ltc_chain::sync_account(
                        &address,
                        &ltc_node,
                        &ltc_network,
                    ) {
                        Ok(state) => {
                            *history_arc.lock().unwrap() = state.history.clone();
                            state.balance_display()
                        }
                        Err(_) => String::from("offline"),
                    };
                    if sender.send_blocking(label).is_err() {
                        break;
                    }
                }
            });

            crate::configuration::ui_channel::attach(
                receiver,
                clone!(
                    #[weak]
                    ltc_balance_arc,
                    #[upgrade_or]
                    ControlFlow::Break,
                    move |price_text| {
                        let mut ltc_balance = ltc_balance_arc.lock().unwrap();
                        if price_text != "Uninitialized" {
                            *ltc_balance = price_text;
                        }

                        ControlFlow::Continue
                    }
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configuration::seed;
    use crate::configuration::wallet_store::StoreSession;

    #[test]
    fn test_generate_btc_wallet() {
        let wallet = ApplicationSettings::generate_btc_wallet(String::from("test_name")).unwrap();
        assert_eq!(wallet.wallet_name.unwrap(), "test_name");
    }

    #[test]
    fn test_generate_eth_wallet() {
        let wallet = ApplicationSettings::generate_eth_wallet(String::from("test_name")).unwrap();
        assert_eq!(wallet.wallet_name.unwrap(), "test_name");
    }

    #[test]
    fn test_generate_ltc_wallet() {
        let wallet = ApplicationSettings::generate_ltc_wallet(String::from("test_name")).unwrap();
        assert_eq!(wallet.wallet_name.unwrap(), "test_name");
        assert!(wallet.address.unwrap().starts_with("ltc1q"));
    }

    #[test]
    fn apply_test_networks_sets_btc_testnet_and_eth_sepolia() {
        let mut settings = ApplicationSettings::new(Tokens::new());
        settings.apply_test_networks(true);
        assert!(settings.is_test_mode());
        assert_eq!(settings.btc_network, "testnet");
        assert_eq!(settings.eth_network, "sepolia");
        assert_eq!(settings.ltc_network, "testnet");
        settings.apply_test_networks(false);
        assert!(!settings.is_test_mode());
        assert_eq!(settings.btc_network, "bitcoin");
        assert_eq!(settings.eth_network, "mainnet");
        assert_eq!(settings.ltc_network, "litecoin");
    }

    #[test]
    fn test_find_config_path() {
        let config_path = ApplicationSettings::find_config_path().unwrap();
        assert_eq!(config_path.file_name().unwrap(), "Config.dic");
        assert!(config_path.parent().unwrap().exists());
        let mut exe_dir = std::env::current_exe().unwrap();
        exe_dir.pop();
        assert_ne!(config_path.parent().unwrap(), exe_dir.as_path());
        assert!(!config_path.to_string_lossy().contains("/Users/andy"));
    }

    #[test]
    fn test_find_error_path() {
        let error_path = ApplicationSettings::find_error_path().unwrap();
        assert_eq!(error_path.file_name().unwrap(), "blockwallet.log");
        assert!(error_path.parent().unwrap().exists());
    }

    #[test]
    fn default_usdc_logo_is_not_hardcoded_dev_path() {
        let icon = crate::configuration::paths::token_icon_path("USDC");
        assert!(!icon.to_string_lossy().contains("/Users/andy"));
    }

    const ABANDON: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn create_flow_confirm_then_derive() {
        let phrase = crate::configuration::onboarding::generate_create_phrase(
            crate::configuration::onboarding::WordCount::Words12,
        )
        .unwrap();
        crate::configuration::onboarding::confirm_created_phrase(&phrase, &phrase).unwrap();
        let mut settings = ApplicationSettings::new(Tokens::new());
        settings.restore_from_mnemonic(&phrase, "").unwrap();
        assert_eq!(settings.mnemonic.as_deref(), Some(phrase.as_str()));
        assert_eq!(settings.btc_wallets[0].mnemonic.as_deref(), Some(phrase.as_str()));
        assert_eq!(settings.eth_wallets[0].mnemonic.as_deref(), Some(phrase.as_str()));
        assert_eq!(settings.sol_wallets[0].mnemonic.as_deref(), Some(phrase.as_str()));
        assert!(settings.btc_wallets[0].address.as_ref().unwrap().starts_with("bc1q"));
    }

    #[test]
    fn apply_eth_network_switches_sepolia_usdc() {
        let mut settings = ApplicationSettings::new(Tokens::new());
        let mainnet_usdc = settings.tokens.eth_tokens["eth:USDC"].address.clone();
        settings.apply_eth_network("sepolia");
        assert_eq!(settings.eth_network, "sepolia");
        let sepolia_usdc = &settings.tokens.eth_tokens["eth:USDC"].address;
        assert_ne!(sepolia_usdc.to_ascii_lowercase(), mainnet_usdc.to_ascii_lowercase());
        assert!(sepolia_usdc.to_ascii_lowercase().starts_with("0x1c7d4b"));
    }

    #[test]
    fn restore_from_mnemonic_shares_seed_across_chains() {
        let mut settings = ApplicationSettings::new(Tokens::new());
        settings.restore_from_mnemonic(ABANDON, "").unwrap();
        assert_eq!(settings.mnemonic.as_deref(), Some(ABANDON));
        assert_eq!(settings.btc_wallets[0].mnemonic.as_deref(), Some(ABANDON));
        assert_eq!(settings.eth_wallets[0].mnemonic.as_deref(), Some(ABANDON));
        assert_eq!(settings.btc_wallets[0].mnemonic, settings.eth_wallets[0].mnemonic);
        assert_eq!(settings.sol_wallets[0].mnemonic.as_deref(), Some(ABANDON));
        assert!(settings.btc_wallets[0].address.as_ref().unwrap().starts_with("bc1q"));
        let eth_address = settings.eth_wallets[0].address.as_ref().unwrap();
        assert!(eth_address.starts_with("0x"));
        assert_eq!(eth_address.len(), 42);
        assert_eq!(settings.eth_wallets[0].path.as_deref(), Some(seed::ETH_PATH));
        assert_eq!(settings.sol_wallets[0].path.as_deref(), Some(seed::SOL_PATH));
        assert!(!settings.sol_wallets[0].address.as_ref().unwrap().is_empty());
        assert_eq!(settings.ltc_wallets[0].mnemonic.as_deref(), Some(ABANDON));
        assert!(settings.ltc_wallets[0].address.as_ref().unwrap().starts_with("ltc1q"));
        assert_eq!(
            settings.btc_wallets[0].address.as_deref(),
            Some("bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu")
        );
        assert_eq!(
            settings.eth_wallets[0].address.as_deref(),
            Some("0x9858EfFD232B4033E47d90003D41EC34EcaEda94")
        );
    }

    #[test]
    fn finish_onboarding_restore_roundtrip_keeps_shared_seed() {
        let mut settings = ApplicationSettings::new(Tokens::new());
        let path = std::env::temp_dir().join(format!(
            "blockwallet-settings-onboard-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        settings.config_path = path.clone();
        settings
            .finish_onboarding(ABANDON, "", "test-password")
            .unwrap();
        assert_eq!(settings.mnemonic.as_deref(), Some(ABANDON));
        assert!(settings.seed_passphrase.is_none());
        let btc = settings.btc_wallets[0].address.clone();
        let eth = settings.eth_wallets[0].address.clone();

        let mut reloaded = ApplicationSettings::new(Tokens::new());
        reloaded.config_path = path.clone();
        reloaded.unlock_store("test-password").unwrap();
        assert_eq!(reloaded.mnemonic.as_deref(), Some(ABANDON));
        assert_eq!(reloaded.btc_wallets[0].address, btc);
        assert_eq!(reloaded.eth_wallets[0].address, eth);
        assert_eq!(
            reloaded.btc_wallets[0].mnemonic.as_deref(),
            reloaded.eth_wallets[0].mnemonic.as_deref()
        );
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn finish_onboarding_passphrase_survives_unlock() {
        let mut settings = ApplicationSettings::new(Tokens::new());
        let path = std::env::temp_dir().join(format!(
            "blockwallet-settings-passphrase-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        settings.config_path = path.clone();
        settings
            .finish_onboarding(ABANDON, "trezor", "test-password")
            .unwrap();
        let btc = settings.btc_wallets[0].address.clone();
        let eth = settings.eth_wallets[0].address.clone();
        assert_eq!(settings.seed_passphrase.as_deref(), Some("trezor"));
        assert_ne!(
            btc.as_deref(),
            Some("bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu")
        );

        let mut reloaded = ApplicationSettings::new(Tokens::new());
        reloaded.config_path = path.clone();
        reloaded.unlock_store("test-password").unwrap();
        assert_eq!(reloaded.seed_passphrase.as_deref(), Some("trezor"));
        assert_eq!(reloaded.btc_wallets[0].address, btc);
        assert_eq!(reloaded.eth_wallets[0].address, eth);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn create_store_derives_btc_and_eth_from_one_seed() {
        let mut settings = ApplicationSettings::new(Tokens::new());
        let path = std::env::temp_dir().join(format!(
            "blockwallet-settings-seed-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        settings.config_path = path.clone();
        settings.create_store("test-password").unwrap();

        let phrase = settings.mnemonic.clone().expect("store seed");
        assert_eq!(phrase.split_whitespace().count(), 12);
        assert_eq!(settings.btc_wallets[0].mnemonic.as_deref(), Some(phrase.as_str()));
        assert_eq!(settings.eth_wallets[0].mnemonic.as_deref(), Some(phrase.as_str()));
        assert_eq!(settings.sol_wallets[0].mnemonic.as_deref(), Some(phrase.as_str()));
        assert_eq!(settings.ltc_wallets[0].mnemonic.as_deref(), Some(phrase.as_str()));
        let btc_address = settings.btc_wallets[0].address.clone();
        let eth_address = settings.eth_wallets[0].address.clone();
        let sol_address = settings.sol_wallets[0].address.clone();
        let ltc_address = settings.ltc_wallets[0].address.clone();
        let payload = settings.to_payload();
        assert_eq!(payload.mnemonic.as_deref(), Some(phrase.as_str()));
        assert!(payload.btc[0].mnemonic.is_none());
        assert!(payload.btc[0].private_key_wif.is_none());
        assert!(payload.eth[0].mnemonic.is_none());
        assert!(payload.eth[0].private_key.is_none());
        assert!(payload.sol[0].mnemonic.is_none());
        assert!(payload.sol[0].private_key.is_none());
        assert!(payload.ltc[0].mnemonic.is_none());
        assert!(payload.ltc[0].private_key_wif.is_none());

        let mut reloaded = ApplicationSettings::new(Tokens::new());
        reloaded.config_path = path.clone();
        reloaded.unlock_store("test-password").unwrap();
        assert_eq!(reloaded.mnemonic.as_deref(), Some(phrase.as_str()));
        assert_eq!(reloaded.btc_wallets[0].mnemonic.as_deref(), Some(phrase.as_str()));
        assert_eq!(reloaded.eth_wallets[0].mnemonic.as_deref(), Some(phrase.as_str()));
        assert_eq!(reloaded.sol_wallets[0].mnemonic.as_deref(), Some(phrase.as_str()));
        assert_eq!(reloaded.ltc_wallets[0].mnemonic.as_deref(), Some(phrase.as_str()));
        assert_eq!(reloaded.btc_wallets[0].address, btc_address);
        assert_eq!(reloaded.eth_wallets[0].address, eth_address);
        assert_eq!(reloaded.sol_wallets[0].address, sol_address);
        assert_eq!(reloaded.ltc_wallets[0].address, ltc_address);
        assert!(reloaded.logged_in);
        assert!(StoreSession::unlock(&path, "wrong").is_err());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn extra_imported_wallet_stays_independent_of_store_seed() {
        let mut settings = ApplicationSettings::new(Tokens::new());
        let path = std::env::temp_dir().join(format!(
            "blockwallet-settings-extra-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        settings.config_path = path.clone();
        settings.create_store("test-password").unwrap();
        let seed_phrase = settings.mnemonic.clone().unwrap();
        let extra = ApplicationSettings::generate_eth_wallet(String::from("imported")).unwrap();
        let extra_mnemonic = extra.mnemonic.clone();
        let extra_address = extra.address.clone();
        assert_ne!(extra_mnemonic.as_deref(), Some(seed_phrase.as_str()));
        settings.eth_wallets.push(extra);
        settings.write_config().unwrap();

        let mut reloaded = ApplicationSettings::new(Tokens::new());
        reloaded.config_path = path.clone();
        reloaded.unlock_store("test-password").unwrap();
        assert_eq!(reloaded.mnemonic.as_deref(), Some(seed_phrase.as_str()));
        assert_eq!(reloaded.eth_wallets.len(), 2);
        assert_eq!(reloaded.eth_wallets[0].mnemonic.as_deref(), Some(seed_phrase.as_str()));
        assert_eq!(reloaded.eth_wallets[1].mnemonic, extra_mnemonic);
        assert_eq!(reloaded.eth_wallets[1].address, extra_address);
        assert_eq!(reloaded.eth_wallets[1].wallet_name.as_deref(), Some("imported"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn wallet_store_roundtrip_preserves_btc_address() {
        let mut settings = ApplicationSettings::new(Tokens::new());
        let path = std::env::temp_dir().join(format!("blockwallet-settings-{}.json", std::process::id()));
        let _ = fs::remove_file(&path);
        settings.config_path = path.clone();
        settings.create_store("test-password").unwrap();
        let address = settings.btc_wallets[0].address.clone();
        let eth_address = settings.eth_wallets[0].address.clone();
        assert_eq!(
            settings.btc_wallets[0].mnemonic.as_deref(),
            settings.eth_wallets[0].mnemonic.as_deref()
        );

        let mut reloaded = ApplicationSettings::new(Tokens::new());
        reloaded.config_path = path.clone();
        reloaded.unlock_store("test-password").unwrap();
        assert_eq!(reloaded.btc_wallets[0].address, address);
        assert_eq!(reloaded.eth_wallets[0].address, eth_address);
        assert_eq!(reloaded.mnemonic, settings.mnemonic);
        assert!(reloaded.logged_in);
        assert!(StoreSession::unlock(&path, "wrong").is_err());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn debug_format_omits_seed_material() {
        let mut settings = ApplicationSettings::new(Tokens::new());
        settings.restore_from_mnemonic(ABANDON, "trezor").unwrap();
        let debug = format!("{settings:?}");
        assert!(!debug.contains("abandon"));
        assert!(!debug.contains("trezor"));
        assert!(!debug.contains("mnemonic"));
    }

    #[test]
    fn lock_store_wipes_in_memory_seed_without_a_session() {
        let mut settings = ApplicationSettings::new(Tokens::new());
        settings.restore_from_mnemonic(ABANDON, "trezor").unwrap();
        assert!(settings.mnemonic.is_some());
        assert!(settings.seed_passphrase.is_some());
        assert!(!settings.btc_wallets.is_empty());
        settings.lock_store();
        assert!(!settings.logged_in);
        assert!(!settings.is_unlocked());
        assert!(settings.store_session.is_none());
        assert!(settings.mnemonic.is_none());
        assert!(settings.seed_passphrase.is_none());
        assert!(settings.btc_wallets.is_empty());
        assert!(settings.eth_wallets.is_empty());
        assert!(settings.sol_wallets.is_empty());
        assert!(settings.ltc_wallets.is_empty());
    }

    #[test]
    fn lock_then_unlock_restores_same_accounts() {
        let mut settings = ApplicationSettings::new(Tokens::new());
        let path = std::env::temp_dir().join(format!(
            "blockwallet-settings-lock-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        settings.config_path = path.clone();
        settings
            .finish_onboarding(ABANDON, "trezor", "test-password")
            .unwrap();
        let btc = settings.btc_wallets[0].address.clone();
        let eth = settings.eth_wallets[0].address.clone();
        let sol = settings.sol_wallets[0].address.clone();
        let ltc = settings.ltc_wallets[0].address.clone();
        assert!(settings.is_unlocked());
        assert!(path.exists());

        settings.lock_store();
        assert!(!settings.is_unlocked());
        assert!(settings.mnemonic.is_none());
        assert!(settings.seed_passphrase.is_none());
        assert!(settings.store_session.is_none());
        assert!(settings.btc_wallets.is_empty());
        assert!(settings.eth_wallets.is_empty());
        assert!(settings.sol_wallets.is_empty());
        assert!(settings.ltc_wallets.is_empty());
        assert!(path.exists());
        assert!(settings.write_config().is_err());
        assert!(settings.unlock_store("wrong").is_err());
        assert!(!settings.is_unlocked());
        assert!(settings.mnemonic.is_none());
        assert!(settings.btc_wallets.is_empty());

        settings.unlock_store("test-password").unwrap();
        assert!(settings.is_unlocked());
        assert_eq!(settings.mnemonic.as_deref(), Some(ABANDON));
        assert_eq!(settings.seed_passphrase.as_deref(), Some("trezor"));
        assert_eq!(settings.btc_wallets[0].address, btc);
        assert_eq!(settings.eth_wallets[0].address, eth);
        assert_eq!(settings.sol_wallets[0].address, sol);
        assert_eq!(settings.ltc_wallets[0].address, ltc);
        assert!(settings.btc_wallets[0].private_key.is_some());
        assert!(settings.eth_wallets[0].private_key.is_some());
        assert!(settings.sol_wallets[0].private_key.is_some());
        assert!(settings.ltc_wallets[0].private_key.is_some());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn verify_password_and_secrets_lookup() {
        let mut settings = ApplicationSettings::new(Tokens::new());
        let path = std::env::temp_dir().join(format!(
            "blockwallet-settings-reveal-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        settings.config_path = path.clone();
        settings
            .finish_onboarding(ABANDON, "", "test-password")
            .unwrap();
        let address = settings.btc_wallets[0].address.clone().unwrap();
        assert!(settings.verify_password("test-password"));
        assert!(!settings.verify_password("wrong"));
        let (mnemonic, key) = settings.secrets_for_address(&address);
        assert_eq!(mnemonic.as_deref(), Some(ABANDON));
        assert!(key.is_some());
        let visible = crate::configuration::wallet_display::default_visible_lines(
            settings.btc_wallets[0].wallet_name.as_deref(),
            settings.btc_wallets[0].address.as_deref(),
        )
        .join("\n");
        assert!(!crate::configuration::wallet_display::visible_text_leaks_secret(&visible, ABANDON));
        assert!(!crate::configuration::wallet_display::visible_text_leaks_secret(
            &visible,
            key.as_deref().unwrap()
        ));
        settings.lock_store();
        assert!(!settings.verify_password("wrong"));
        assert!(settings.secrets_for_address(&address).0.is_none());
        let _ = fs::remove_file(&path);
    }
}