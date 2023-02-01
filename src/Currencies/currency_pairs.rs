use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;
use glib::{clone, Continue, MainContext, PRIORITY_DEFAULT, ObjectType};
use crate::configuration::*;

#[derive(Clone, Debug)]
pub struct CurrencyPairs {
    pub default_currency    : Option<Token>,
    pub pairs               : Vec<((Token, Token), Arc<Mutex<String>>)>,
    pub btc_usd             : Arc<Mutex<String>>
}

impl CurrencyPairs {
    pub fn new() -> CurrencyPairs {
        let mut pairs = Vec::new();
        pairs.push(((Token::BTC, Token::USDC), Arc::new(Mutex::new(String::from("Uninitialized")))));
        pairs.push(((Token::ETH, Token::USDC), Arc::new(Mutex::new(String::from("Uninitialized")))));
        pairs.push(((Token::MATIC, Token::USDC), Arc::new(Mutex::new(String::from("Uninitialized")))));
        pairs.push(((Token::WBTC, Token::USDC), Arc::new(Mutex::new(String::from("Uninitialized")))));
        pairs.push(((Token::UNI, Token::USDC), Arc::new(Mutex::new(String::from("Uninitialized")))));

        CurrencyPairs {
            default_currency    : Some(Token::USDC),
            pairs               : pairs,
            btc_usd             : Arc::new(Mutex::new(String::from("Uninitialized"))),
        }
    }

    pub async fn get_btc_price() -> Result<String, block_error::Error> {
        let resp = match reqwest::get("https://api.kine.exchange/market/api/price/BTCUSD").await?.text().await {
            Ok(r)  => r,
            Err(_) => return Ok(String::from("Uninitialized"))
        };
        
        let json: Value = match serde_json::from_str(&resp) {
            Ok(r)  => r,
            Err(_) => return Ok(String::from("Uninitialized"))
        };

        return Ok(json["data"]["price"].to_string());
    }

    pub async fn get_currency_price() -> Result<String, block_error::Error> {
        let resp = match reqwest::get("https://api.0x.org/swap/v1/quote?buyToken=USDC&sellToken=ETH&sellAmount=100000000000000000").await?.text().await {
            Ok(r)  => r,
            Err(_) => return Ok(String::from("Uninitialized"))
        };

        let json: Value = match serde_json::from_str(&resp) {
            Ok(r)  => r,
            Err(_) => return Ok(String::from("Uninitialized"))
        };

        return Ok(json["price"].to_string());
    }

    pub fn update_token_balances(&self) {
        let pairs = self.pairs.clone();
        
        thread::spawn(move || {
            let len = pairs.len();
            loop {
                for i in 0..len {
                    let current_price = Arc::clone(&pairs[i].1);
                    let token         = pairs[i].0.0.clone();
                    thread::spawn(move || {
                        let runtime = tokio::runtime::Runtime::new().unwrap();
                        let _ = runtime.block_on(runtime.spawn(async move {
                            let currency_quote;
                            if token == Token::BTC {
                                currency_quote = match CurrencyPairs::get_btc_price().await {
                                    Ok(quote) => quote,
                                    Err(_)    => String::from("Uninitialized")
                                };
                            }
                            else {
                                currency_quote = match CurrencyPairs::get_currency_price().await {
                                    Ok(quote) => quote,
                                    Err(_)    => String::from("Uninitialized")
                                };
                            }

                            *current_price.lock().unwrap() = currency_quote;
                        }));
                    });
                }
                thread::sleep(Duration::from_secs(1));
            }
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Token {
    USDC,
    BTC,
    ETH,
    MATIC,
    WBTC,
    UNI
}

impl Token {
    pub fn ticker(&self) -> &'static str {
        match self {
            Token::USDC  => "USDC",
            Token::BTC   => "BTC",
            Token::ETH   => "ETH",
            Token::MATIC => "MATIC",
            Token::WBTC  => "WBTC",
            Token::UNI   => "UNI",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Token::USDC  => "USD Coin",
            Token::BTC   => "Bitcoin",
            Token::ETH   => "Ethereum",
            Token::MATIC => "Polygon",
            Token::WBTC  => "Wrapped Bitcoin",
            Token::UNI   => "Uniswap",
        }
    }
}