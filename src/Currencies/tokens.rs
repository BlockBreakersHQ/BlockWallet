use colored::Colorize;
use std::path::PathBuf;
use std::fmt;
use std::fmt::Display;

use crate::ApplicationSettings;

#[derive(Clone, Debug)]
pub struct Tokens {
    pub tokens: Vec<Token>
}

impl Tokens {
    pub fn new() -> Self {
        let mut btc_path = match ApplicationSettings::find_images_path(){
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

        let mut tokens = Vec::new();
        tokens.push(t);

        Tokens {
            tokens: tokens
        }
    }
    /*
    pub fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut output = String::new();
        for currency in self.tokens.iter() {
            output.push_str(&format!("{}\n", currency.display()));
        }
        write!(f, "{}", output)
    }
    */
    pub fn len(&self) -> usize {
        self.tokens.len()
    }
}

#[derive(Clone, Debug)]
pub struct Token {
    pub name    : String,
    pub symbol  : String,
    pub address : String,
    pub logo    : PathBuf,
    pub decimals: i32
}

impl Token {
    pub fn new(name: String, ticker: String, address: String, logo: PathBuf, digits: i32, starred: bool) -> Self {
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

