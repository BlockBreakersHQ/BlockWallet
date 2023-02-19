use std::{env, io, thread, fs};
use std::io::{Error, ErrorKind, Write, Read, Seek, SeekFrom};
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::collections::HashMap;
use chrono;
use cocoon::{Cocoon};
use glib::{clone, Continue, MainContext, PRIORITY_DEFAULT};
use std::sync::Arc;

use crate::currencies::eth;
use crate::currencies::eth::EthereumWallet;
use crate::currencies::btc;
use crate::currencies::btc::BitcoinWallet;
use crate::currencies::tokens::*;
use crate::configuration::block_error;

#[derive(Clone, Debug)]
pub struct ApplicationSettings {
    pub config_path   : PathBuf,
    pub error_path    : PathBuf,
    pub user_hash     : String,
    pub btc_wallets   : Vec<BitcoinWallet>,
    pub eth_wallets   : Vec<EthereumWallet>,
    pub tokens        : Tokens,
    pub starred       : HashMap<String, Token>,
    pub logged_in     : bool,
    pub infura_key    : String,
    pub etherscan_key : String
}

impl ApplicationSettings {
    pub fn new(tokens: Tokens) -> Self {
        let cpath = ApplicationSettings::find_config_path().unwrap();
        let epath = ApplicationSettings::find_error_path().unwrap();

        let hash = String::new();

        let mut bitcoin_wallets = vec![];
        let mut ethereum_wallets = vec![];

        if !std::path::Path::new(&epath).exists() {
            match File::create(&epath) {
                Err(why) => panic!("couldn't create {}: {}", epath.display(), why),
                Ok(_) => ()
            };
        }

        let mut starred = HashMap::new();
        let mut i_key = String::new();
        let mut e_key = String::new();

        if !std::path::Path::new(&cpath).exists() {
            let b_wallet = ApplicationSettings::generate_btc_wallet(String::new());
            let e_wallet = ApplicationSettings::generate_eth_wallet(String::new());

            bitcoin_wallets.push(b_wallet);
            ethereum_wallets.push(e_wallet);

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
                }
            }

            let mut ypath = match env::current_exe() {
                Ok(path) => path,
                Err(why) => panic!("couldn't get executable directory: {}", why)
            };
            ypath.pop();
            ypath.push("Config.yml");

            if !ypath.exists() {
                match File::create(&ypath) {
                    Ok(_) => {
                        let data = "INFURA_KEY=\nETHERSCAN_KEY=\n";
                        fs::write(ypath, data).expect("Unable to write file");
                    },
                    Err(why) => panic!("couldn't create {}: {}", &ypath.clone().display(), why)
                };
            } else {
                let printable_path = ypath.clone();
                let content = match fs::read_to_string(ypath) {
                    Ok(content) => content,
                    Err(why) => panic!("couldn't read {}: {}", printable_path.display(), why)
                };

                let contents: Vec<&str> = content.split("\n").collect();
    
                for i in 0..contents.len() {
                    if contents[i].contains("INFURA_KEY=") {
                        i_key = contents[i].split("=").collect::<Vec<&str>>()[1].to_string();
                    } else if contents[i].contains("ETHERSCAN_KEY=") {
                        e_key = contents[i].split("=").collect::<Vec<&str>>()[1].to_string();
                    }
                }
            }
        }

        ApplicationSettings {
            config_path   : cpath,
            error_path    : epath,
            user_hash     : hash,
            btc_wallets   : bitcoin_wallets,
            eth_wallets   : ethereum_wallets,
            tokens        : tokens,
            starred       : starred,
            logged_in     : false,
            infura_key    : i_key,
            etherscan_key : e_key
        }
    }

    pub fn generate_btc_wallet(wallet_name: String) -> BitcoinWallet {

        let btc_wallet = match btc::generate_btc_hd_wallet() {
            Some(wallet) => Some(wallet),
            None => None
        };

        let mut bitcoin_wallet = match btc_wallet {
            Some(wallet) => wallet,
            None => panic!("An error has occurred creating a Bitcoin wallet. See error log for details: {}", ApplicationSettings::find_error_path().unwrap().display())
        };

        if wallet_name.is_empty() {
            bitcoin_wallet.set_wallet_name(String::from("btc_wallet"));
        }
        else {
            bitcoin_wallet.set_wallet_name(wallet_name);
        }

        return bitcoin_wallet;
    }

    pub fn generate_eth_wallet(wallet_name: String) -> EthereumWallet {
        let eth_wallet = match eth::generate_eth_hd_wallet() {
            Some(wallet) => Some(wallet),
            None => None
        };

        let mut ethereum_wallet = match eth_wallet {
            Some(wallet) => wallet,
            None => panic!("An error has occurred creating a Ethereum wallet. See error log for details: {}", ApplicationSettings::find_error_path().unwrap().display())
        };

        if wallet_name.is_empty() {
            ethereum_wallet.set_wallet_name(String::from("eth_wallet"));
        }
        else {
            ethereum_wallet.set_wallet_name(wallet_name);
        }
        
        return ethereum_wallet;
    }

    pub fn find_config_path() -> io::Result<PathBuf> {
        let mut cpath = env::current_exe()?;
        cpath.pop();
        cpath.push("Config.dic");
        Ok(cpath)
    }

    pub fn find_error_path() -> io::Result<PathBuf> {
        let mut cpath = env::current_exe()?;
        cpath.pop();
        cpath.push("Error.log");
        Ok(cpath)
    }

    pub fn find_images_path() -> io::Result<PathBuf> {
        let mut ipath = env::current_exe()?;
        ipath.pop();
        ipath.push("Images");
        Ok(ipath)
    }

    pub fn read_config(&mut self) -> Result<String, block_error::Error> {
        if !Path::new(&self.config_path).exists() {
            self.logged_in = true;
            return Ok(String::new());
        }

        let mut file = File::options()
            .read(true)
            .open(&self.config_path)?;

        let mut contents = vec![];
        file.read_to_end(&mut contents)?;
        
        if contents.len() <= 0 || !Path::new(&self.config_path).exists() {
            let b_wallet = ApplicationSettings::generate_btc_wallet(String::new());
            let e_wallet = ApplicationSettings::generate_eth_wallet(String::new());

            self.btc_wallets.push(b_wallet);
            self.eth_wallets.push(e_wallet);

            return Ok(String::new());
        }

        if self.logged_in == false {
            self.btc_wallets = vec![];
            self.eth_wallets = vec![];
        }
        
        file.seek(SeekFrom::Start(0))?;
        let hash = &self.user_hash;
        let cocoon = Cocoon::new(hash.as_bytes());
        let encrypted_file_content = cocoon.parse(&mut file)?;
        let file_content = std::str::from_utf8(&encrypted_file_content)?.to_string();
        self.logged_in = true;
        
        let content: Vec<&str> = file_content.split("<Entry>\n").collect();
        let mut modified_content = Vec::new();

        for i in content {
            if i.len() > 10 {
                modified_content.push(i);
            }
        }
        
        let mut settings    = Vec::new();
        let mut btc_wallets = Vec::new();
        let mut eth_wallets = Vec::new();

        for i in modified_content {
            if i.contains("Sector: Settings") {
                settings = i.split("Starred______\n").collect();
                settings.remove(0);
            }
            else if i.contains("Sector: Bitcoin") {
                btc_wallets = i.split("Wallet______").collect();
                btc_wallets.remove(0);
            }
            else if i.contains("Sector: Ethereum") {
                eth_wallets = i.split("Wallet______").collect();
                eth_wallets.remove(0);
            }
        }

        if settings.len() <= 0 {
            for (key, value) in self.tokens.eth_tokens.clone() {
                if self.tokens.eth_tokens[&key].symbol == "BTC" {
                    self.starred.insert(key.clone(), value.clone());
                } else if self.tokens.eth_tokens[&key].symbol == "ETH" {
                    self.starred.insert(key.clone(), value.clone());
                } else if self.tokens.eth_tokens[&key].symbol == "MATIC" {
                    self.starred.insert(key.clone(), value.clone());
                } else if self.tokens.eth_tokens[&key].symbol == "WBTC" {
                    self.starred.insert(key.clone(), value.clone());
                } else if self.tokens.eth_tokens[&key].symbol == "UNI" {
                    self.starred.insert(key.clone(), value.clone());
                } else if self.tokens.eth_tokens[&key].symbol == "BNB" {
                    self.starred.insert(key.clone(), value.clone());
                } else if self.tokens.eth_tokens[&key].symbol == "SHIB" {
                    self.starred.insert(key.clone(), value.clone());
                } else if self.tokens.eth_tokens[&key].symbol == "TRON" {
                    self.starred.insert(key.clone(), value.clone());
                } else if self.tokens.eth_tokens[&key].symbol == "LINK" {
                    self.starred.insert(key.clone(), value.clone());
                } else if self.tokens.eth_tokens[&key].symbol == "QNT" {
                    self.starred.insert(key.clone(), value.clone());
                } else if self.tokens.eth_tokens[&key].symbol == "APE" {
                    self.starred.insert(key.clone(), value.clone());
                } else if self.tokens.eth_tokens[&key].symbol == "FTM" {
                    self.starred.insert(key.clone(), value.clone());
                } else if self.tokens.eth_tokens[&key].symbol == "GRT" {
                    self.starred.insert(key.clone(), value.clone());
                } else if self.tokens.eth_tokens[&key].symbol == "SAND" {
                    self.starred.insert(key.clone(), value.clone());
                } else if self.tokens.eth_tokens[&key].symbol == "MANA" {
                    self.starred.insert(key.clone(), value.clone());
                } else if self.tokens.eth_tokens[&key].symbol == "AXS" {
                    self.starred.insert(key.clone(), value.clone());
                } else if self.tokens.eth_tokens[&key].symbol == "CHZ" {
                    self.starred.insert(key.clone(), value.clone());
                } else if self.tokens.eth_tokens[&key].symbol == "CRV" {
                    self.starred.insert(key.clone(), value.clone());
                }

                let mut i_key = String::new();
                let mut e_key = String::new();

                let mut ypath = match env::current_exe() {
                    Ok(path) => path,
                    Err(why) => panic!("couldn't get executable directory: {}", why)
                };
                ypath.pop();
                ypath.push("Config.yml");
    
                if !ypath.exists() {
                    match File::create(&ypath) {
                        Ok(_) => {
                            let data = "INFURA_KEY=\nETHERSCAN_KEY=\n";
                            fs::write(ypath, data).expect("Unable to write file");
                        },
                        Err(why) => panic!("couldn't create {}: {}", &ypath.clone().display(), why)
                    };
                } else {
                    let printable_path = ypath.clone();
                    let content = match fs::read_to_string(ypath) {
                        Ok(content) => content,
                        Err(why) => panic!("couldn't read {}: {}", printable_path.display(), why)
                    };
    
                    let contents: Vec<&str> = content.split("\n").collect();
        
                    for i in 0..contents.len() {
                        if contents[i].contains("INFURA_KEY=") {
                            i_key = contents[i].split("=").collect::<Vec<&str>>()[1].to_string();
                        } else if contents[i].contains("ETHERSCAN_KEY=") {
                            e_key = contents[i].split("=").collect::<Vec<&str>>()[1].to_string();
                        }
                    }
                }
                self.infura_key = i_key;
                self.etherscan_key = e_key;
            }
        } else {
            for setting in settings {
                let t: Vec<&str> = setting.split("\n").collect();
                for i in t {
                    if i.len() <= 0 {
                        continue;
                    }
                    if i.contains("ETHERSCAN_KEY=") {
                        self.etherscan_key = i.split("=").collect::<Vec<&str>>()[1].to_string();
                    } else if i.contains("ETH") {
                        let symbol = i.replace("      ETH TOKEN:", "");
                        for (key, value) in self.tokens.eth_tokens.clone() {
                            if self.tokens.eth_tokens[&key].symbol == symbol {
                                self.starred.insert(key.clone(), value.clone());
                            }
                        }
                    } else if i.contains("INFURA_KEY=") {
                        self.infura_key = i.split("=").collect::<Vec<&str>>()[1].to_string();
                    }
                }
            }
        }
        
        if btc_wallets.len() > 0 {
            for btcw in &btc_wallets {
                let attributes: Vec<&str>    = btcw.split("\n").collect();
                let mut path                 = String::new();
                let mut mnemonic             = String::new();
                let mut private_key          = String::new();
                let mut extended_private_key = String::new();
                let mut wallet_name          = String::new();

                for i in attributes {
                    if i.contains("Path") {
                        let element: Vec<&str> = i.split("Path").collect();
                        let mut unmodified_path = String::from(element[1]);
                        unmodified_path.retain(|c| !c.is_whitespace());
                        path = String::from(&unmodified_path[4..unmodified_path.len()]);
                    }
                    if i.contains("Mnemonic") {
                        let element: Vec<&str> = i.split("Mnemonic").collect();
                        let unmodified_mnemonic = String::from(element[1]);
                        let unmod_mnemonic = unmodified_mnemonic.split_whitespace();
                        let mut m = String::new();
                        for j in unmod_mnemonic {
                            m += j;
                            m += " ";
                        }
                        m.pop();
                        mnemonic = String::from(&m[5..m.len()]);
                    }
                    if i.contains("Extended Private Key") {
                        let element: Vec<&str> = i.split("Extended Private Key").collect();
                        let mut unmodified_extended_private_key = String::from(element[1]);
                        unmodified_extended_private_key.retain(|c| !c.is_whitespace());
                        extended_private_key = String::from(&unmodified_extended_private_key[4..unmodified_extended_private_key.len()]);
                    }
                    else if i.contains("Private Key") {
                        let element: Vec<&str> = i.split("Private Key").collect();
                        let mut unmodified_private_key = String::from(element[1]);
                        unmodified_private_key.retain(|c| !c.is_whitespace());
                        private_key = String::from(&unmodified_private_key[4..unmodified_private_key.len()]);
                    }

                    if i.contains("Wallet Name") {
                        let element: Vec<&str> = i.split("Wallet Name").collect();
                        let mut unmodified_wallet_name = String::from(element[1]);
                        unmodified_wallet_name.retain(|c| !c.is_whitespace());
                        wallet_name = String::from(&unmodified_wallet_name[4..unmodified_wallet_name.len()]);
                    }
                }

                if !mnemonic.is_empty() {
                    let wallet = btc::generate_from_mnemonic(&mnemonic, &path);

                    let mut b_wallet = match wallet {
                        Some(w) => w,
                        None => panic!("ERROR: generating Bitcoin hd wallet from mnemonic failed.")
                    };
                    if wallet_name.len() > 0 {
                        b_wallet.set_wallet_name(wallet_name);
                    } else {
                        b_wallet.set_wallet_name(String::from("Bitcoin Wallet"));
                    }
                    let _ = &self.btc_wallets.push(b_wallet);
                }
                else if !private_key.is_empty() {
                    let wallet = btc::generate_from_private_key(&private_key);

                    let mut b_wallet = match wallet {
                        Some(w) => w,
                        None => panic!("ERROR: generating Bitcoin hd wallet from private key failed.")
                    };
                    if wallet_name.len() > 0 {
                        b_wallet.set_wallet_name(wallet_name);
                    } else {
                        b_wallet.set_wallet_name(String::from("Bitcoin Wallet"));
                    }
                    let _ = &self.btc_wallets.push(b_wallet);
                }
                else if !extended_private_key.is_empty() {
                    let wallet = btc::generate_from_extended_private_key(&extended_private_key, &path);

                    let mut b_wallet = match wallet {
                        Some(w) => w,
                        None => panic!("ERROR: generating Bitcoin hd wallet from extended private key failed.")
                    };
                    if wallet_name.len() > 0 {
                        b_wallet.set_wallet_name(wallet_name);
                    } else {
                        b_wallet.set_wallet_name(String::from("Bitcoin Wallet"));
                    }
                    let _ = &self.btc_wallets.push(b_wallet);
                }
            }
        } else {
            let b_wallet = ApplicationSettings::generate_btc_wallet(String::new());
            self.btc_wallets.push(b_wallet);
        }

        if eth_wallets.len() > 0 {
            for ethw in &eth_wallets {
                let attributes: Vec<&str>    = ethw.split("\n").collect();
                let mut path                 = String::new();
                let mut mnemonic             = String::new();
                let mut private_key          = String::new();
                let mut extended_private_key = String::new();
                let mut wallet_name          = String::new();
                
                for i in attributes {
                    if i.contains("Path") {
                        let element: Vec<&str> = i.split("Path").collect();
                        let mut unmodified_path = String::from(element[1]);
                        unmodified_path.retain(|c| !c.is_whitespace());
                        path = String::from(&unmodified_path[4..unmodified_path.len()]);
                    }
                    if i.contains("Mnemonic") {
                        let element: Vec<&str> = i.split("Mnemonic").collect();
                        let unmodified_mnemonic = String::from(element[1]);
                        let unmod_mnemonic = unmodified_mnemonic.split_whitespace();
                        let mut m = String::new();
                        for j in unmod_mnemonic {
                            m += j;
                            m += " ";
                        }
                        m.pop();
                        mnemonic = String::from(&m[5..m.len()]);
                    }
                    if i.contains("Extended Private Key") {
                        let element: Vec<&str> = i.split("Extended Private Key").collect();
                        let mut unmodified_extended_private_key = String::from(element[1]);
                        unmodified_extended_private_key.retain(|c| !c.is_whitespace());
                        extended_private_key = String::from(&unmodified_extended_private_key[4..unmodified_extended_private_key.len()]);
                    }
                    else if i.contains("Private Key") {
                        let element: Vec<&str> = i.split("Private Key").collect();
                        let mut unmodified_private_key = String::from(element[1]);
                        unmodified_private_key.retain(|c| !c.is_whitespace());
                        private_key = String::from(&unmodified_private_key[4..unmodified_private_key.len()]);
                    }

                    if i.contains("Wallet Name") {
                        let element: Vec<&str> = i.split("Wallet Name").collect();
                        let mut unmodified_wallet_name = String::from(element[1]);
                        unmodified_wallet_name.retain(|c| !c.is_whitespace());
                        wallet_name = String::from(&unmodified_wallet_name[4..unmodified_wallet_name.len()]);
                    }
                }

                if !mnemonic.is_empty() {
                    let wallet = eth::generate_from_mnemonic(&mnemonic, &path);

                    let mut e_wallet = match wallet {
                        Some(w) => w,
                        None => panic!("ERROR: generating Ethereum hd wallet from mnemonic failed.")
                    };
                    if wallet_name.len() > 0 {
                        e_wallet.set_wallet_name(wallet_name);
                    } else {
                        e_wallet.set_wallet_name(String::from("Ethereum Wallet"));
                    }
                    let _ = &self.eth_wallets.push(e_wallet);
                }
                else if !private_key.is_empty() {
                    let wallet = eth::generate_from_private_key(&private_key);

                    let mut e_wallet = match wallet {
                        Some(w) => w,
                        None => panic!("ERROR: generating Ethereum hd wallet from private key failed.")
                    };
                    if wallet_name.len() > 0 {
                        e_wallet.set_wallet_name(wallet_name);
                    } else {
                        e_wallet.set_wallet_name(String::from("Ethereum Wallet"));
                    }
                    let _ = &self.eth_wallets.push(e_wallet);
                }
                else if !extended_private_key.is_empty() {
                    let wallet = eth::generate_from_extended_private_key(&extended_private_key, &path);

                    let mut e_wallet = match wallet {
                        Some(w) => w,
                        None => panic!("ERROR: generating Ethereum hd wallet from extended private key failed.")
                    };
                    if wallet_name.len() > 0 {
                        e_wallet.set_wallet_name(wallet_name);
                    } else {
                        e_wallet.set_wallet_name(String::from("Ethereum Wallet"));
                    }
                    let _ = &self.eth_wallets.push(e_wallet);
                }
            }
        } else {
            let e_wallet = ApplicationSettings::generate_eth_wallet(String::new());
            self.eth_wallets.push(e_wallet);
        }
        Ok(file_content)
    }

    pub fn write_config(&mut self) -> Result<bool, Error> {
        let mut output = String::new();
        if &self.starred.len() > &0 || &self.infura_key.len() > &0 || &self.etherscan_key.len() > &0 {
            output += "<Entry>\n      Sector: Settings\n";
            if &self.starred.len() > &0 {
                output += &format!("      Starred______\n");
                for (key, value) in &self.starred {
                    output += &format!("      ETH TOKEN:{}\n", value.symbol);
                }
            }
            if &self.infura_key.len() > &0 {
                output += &format!("      INFURA_KEY={}\n", self.infura_key);
            }
            if &self.etherscan_key.len() > &0 {
                output += &format!("      ETHERSCAN_KEY={}\n", self.etherscan_key);
            }
            output += "<Entry>\n";
        }
        if &self.btc_wallets.len() > &0 {
            output += "<Entry>\n      Sector: Bitcoin\n";
            for (item, i) in (&self.btc_wallets).iter().enumerate() {
                output += &format!("      Wallet______ {}", item);
                output += &format!("{}\n", i);
            }
            output += "<Entry>\n";
        }
        if &self.eth_wallets.len() > &0 {
            output += "<Entry>\n      Sector: Ethereum\n";
            for (item, i) in (&self.eth_wallets).iter().enumerate() {
                output += &format!("      Wallet______ {}", item);
                output += &format!("{}\n", i);
            }
            output += "<Entry>\n";
        }

        let hash = &self.user_hash;

        let cocoon = Cocoon::new(hash.as_bytes());
        let out_vec: Vec<u8> = output.as_bytes().to_vec();

        let mut file = match File::options().create(true).write(true).open(&self.config_path) {
            Ok(f) => f,
            Err(e) => {
                self.write_error(format!("ERROR: error encountered when opening file: {}", e));
                eprintln!("ERROR: {}",e);
                return Err(e);
            }
        };

        match cocoon.dump(out_vec, &mut file) {
            Ok(_) => return Ok(true),
            Err(e) => {
                self.write_error(format!("ERROR: error encountered when writing to file: {:?}", e));
                return Err(std::io::Error::new(ErrorKind::InvalidData, format!("{:?}", e)));
            }
        }
    }

    pub fn write_error(&self, err: String) {
        match OpenOptions::new().append(true).open(&self.error_path) {
            Ok(mut file)  => {
                write!(file, "ERROR ({}): {}", chrono::offset::Local::now(), err).expect("unable to write");
            }
            Err(e)     => {
                eprintln!("ERROR ({}): Unable to open {}: {}", chrono::offset::Local::now(), self.error_path.display(), e);
            }
        };
    }

    pub fn write_error_to_path(pathbuf: &PathBuf, err: String) {
        let mut path = PathBuf::new();
        if pathbuf ==  &path {
            path = ApplicationSettings::find_error_path().unwrap();
        } else {
            path = pathbuf.to_path_buf();
        }

        match OpenOptions::new().append(true).open(path.clone()) {
            Ok(mut file)  => {
                write!(file, "ERROR ({}): {}", chrono::offset::Local::now(), err).expect("unable to write");
            }
            Err(e)     => {
                eprintln!("ERROR ({}): Unable to open {}: {}", chrono::offset::Local::now(), path.display(), e);
            }
        };
    }

    pub fn update_balances(&self) {
        let mut run_before = false;

        for i in 0..self.btc_wallets.len() {
            let btc_balance_arc = match &self.btc_wallets[i].balance {
                Some(b) => Arc::clone(b),
                None    => panic!("ERROR: failed aquiring balance mutex.")
            };
    
            let address = match &self.btc_wallets[i].address {
                Some(b) => String::from(b),
                None    => String::from("Uninitialized")
            };
    
            let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);
    
            thread::spawn(move || {
                loop {
                    let runtime = tokio::runtime::Runtime::new().unwrap();
                    let sender  = sender.clone();
                    let address = address.clone();
    
                    if run_before == false {
                        thread::sleep(Duration::from_secs(1));
                        run_before = true;
                    }
                    else {
                        thread::sleep(Duration::from_secs(10));
                    }
    
                    let _ = runtime.block_on(runtime.spawn(async move {
                        let btc_price = match BitcoinWallet::get_balance(address).await {
                            Some(label)  => label,
                            None         => String::from("Uninitialized")
                        };
    
                        match sender.send(btc_price) {
                            Ok(_) => (),
                            Err(e) => {
                                let epath = ApplicationSettings::find_error_path().unwrap();
                                ApplicationSettings::write_error_to_path(&epath, format!("ERROR: Sending through channel: Home page: {}", e));
                            }
                        };
                    }));
                }
            });

            receiver.attach(
                None,
                clone!(@weak btc_balance_arc => @default-return Continue(false),
                    move |price_text| {
                        let mut btc_balance = btc_balance_arc.lock().unwrap();
                        if price_text != "Uninitialized" {
                            *btc_balance = price_text;
                        }

                        Continue(true)
                    }
                ),
            );
        }

        for i in 0..self.eth_wallets.len() {
            let etherscan_key = String::from(&self.etherscan_key.clone());
            let eth_balance_arc = match &self.eth_wallets[i].balance {
                Some(b) => Arc::clone(b),
                None    => panic!("ERROR: failed aquiring balance mutex.")
            };
    
            let address = match &self.eth_wallets[i].address {
                Some(b) => String::from(b),
                None    => String::from("Uninitialized")
            };

            let ethw = self.eth_wallets[i].clone();
    
            let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);
    
            thread::spawn(move || {
                loop {
                    let runtime = tokio::runtime::Runtime::new().unwrap();
                    let sender  = sender.clone();
                    let address = address.clone();
                    let etherscan_key = etherscan_key.clone();
                    let mut ethw = ethw.clone();
    
                    if run_before == false {
                        thread::sleep(Duration::from_secs(1));
                        run_before = true;
                    }
                    else {
                        thread::sleep(Duration::from_secs(10));
                    }
    
                    let _ = runtime.block_on(runtime.spawn(async move {
                        let eth_price = match EthereumWallet::get_balance(address, etherscan_key.clone()).await {
                            Some(label)  => label,
                            None         => String::from("Uninitialized")
                        };
                        ethw.get_erc20_balances(etherscan_key.clone()).await;
                        sender.send(eth_price).expect("Could not send through channel");
                    }));
                }
            });

            receiver.attach(
                None,
                clone!(@weak eth_balance_arc => @default-return Continue(false),
                    move |price_text| {
                        let mut eth_balance = eth_balance_arc.lock().unwrap();
                        if price_text != "Uninitialized" {
                            *eth_balance = price_text;
                        }

                        Continue(true)
                    }
                ),
            );
        }
    }
}
