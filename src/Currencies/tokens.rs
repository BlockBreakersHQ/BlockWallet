use colored::Colorize;
use std::path::PathBuf;
use std::fmt;
use std::fmt::Display;
use std::collections::HashMap;
use serde::Serialize;

use crate::ApplicationSettings;

#[derive(Clone, Debug)]
pub struct Tokens {
    pub eth_tokens: HashMap<String, Token>
}

impl Tokens {
    pub fn new() -> Self {
        let btc_path = match ApplicationSettings::find_images_path(){
            Ok(mut bp) => {
                bp.push("Icons/btc.png");
                bp
            },
            Err(_) => PathBuf::new()
        };

        let t = Token {
            name    : String::from("Bitcoin"),
            symbol  : String::from("BTC"),
            address : String::from("0x0000000000000000000000000000000000000000"),
            logo    : btc_path,
            decimals: 8
        };

        let mut eth_tokens = HashMap::new();
        eth_tokens.insert(String::from("BTC"), t);

        Tokens {
            eth_tokens: eth_tokens
        }
    }

    pub fn len(&self) -> usize {
        self.eth_tokens.len()
    }
}

impl Display for Tokens {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut output = String::new();
        for (_key, value) in &self.eth_tokens {
            output.push_str(&format!("{}\n", value));
        }
        write!(f, "{}", output)
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct Token {
    pub name    : String,
    pub symbol  : String,
    pub address : String,
    pub logo    : PathBuf,
    pub decimals: i32
}

impl Token {
    pub fn new(name: String, ticker: String, address: String, logo: PathBuf, digits: i32) -> Self {
        Token {
            name    : name,
            symbol  : ticker,
            address : address,
            logo    : logo,
            decimals: digits
        }
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let output = [
            format!("      {}              {}\n", "Name".cyan().bold(), self.name),
            format!("      {}            {}\n", "Ticker".cyan().bold(), self.symbol),
            format!("      {}           {}\n", "Address".cyan().bold(), self.address),
            format!("      {}              {}\n", "logo".cyan().bold(), self.logo.display()),
            format!("      {}            {}\n", "Digits".cyan().bold(), self.decimals),
        ]
        .concat();
        
        write!(f, "{}", output)
    }
}

