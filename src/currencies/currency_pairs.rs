use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;
use crate::configuration::*;
use crate::currencies::tokens::*;
use crate::ApplicationSettings;

#[derive(Clone, Debug)]
pub struct CurrencyPairs {
    pub default_currency    : Token,
    pub pairs               : Vec<(Token, Arc<Mutex<String>>)>,
}

impl CurrencyPairs {
    pub fn new(app_settings: ApplicationSettings) -> CurrencyPairs {
        let mut pairs: Vec<(Token, Arc<Mutex<String>>)> = Vec::new();
        let default = app_settings.default_currency;

        for (_key, value) in app_settings.starred {
            pairs.push((value, Arc::new(Mutex::new(String::from("Uninitialized")))));
        }

        CurrencyPairs {
            default_currency    : default,
            pairs               : pairs
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

        return Ok(json["data"]["price"].to_string().replace("\"", ""));
    }

    pub async fn get_currency_price(from_token: Token, to_token: Token) -> Result<String, block_error::Error> {
        let digits = CurrencyPairs::get_exponent(from_token.decimals);

        let url = format!("https://api.1inch.io/v5.0/1/quote?fromTokenAddress={}&toTokenAddress={}&amount={}", from_token.address, to_token.address, digits.to_string());
        let resp = match reqwest::get(url).await?.text().await {
            Ok(r)  => r,
            Err(_) => return Ok(String::from("Uninitialized"))
        };

        let json: Value = match serde_json::from_str(&resp) {
            Ok(r)  => r,
            Err(_) => return Ok(String::from("Uninitialized"))
        };

        let token_amount = json["toTokenAmount"].to_string().replace("\"", "");
        
        let token_float  = match token_amount.parse::<i64>() {
            Ok(r)  => r,
            Err(_) => return Ok(String::from("Uninitialized"))
        };
        
        let token_final  = token_float as f64 / CurrencyPairs::get_exponent(to_token.decimals);
        if to_token.symbol == "USDC" {
            return Ok(format!("${:.5}", token_final));
        }
        
        return Ok(token_final.to_string());
    }

    pub fn update_token_balances(&self) {
        let pairs = self.pairs.clone();
        let default = self.default_currency.clone();
        
        thread::spawn(move || {
            let len = pairs.len();
            loop {
                for i in 0..len {
                    let current_price = pairs[i].1.clone();
                    let token         = pairs[i].0.clone();
                    let default       = default.clone();
                    thread::spawn(move || {
                        let runtime = tokio::runtime::Runtime::new().unwrap();
                        let _ = runtime.block_on(runtime.spawn(async move {
                            let currency_quote;
                            if token.symbol == "BTC" {
                                currency_quote = match CurrencyPairs::get_btc_price().await {
                                    Ok(quote) => quote,
                                    Err(_)    => String::from("Uninitialized")
                                };
                            } else {
                                currency_quote = match CurrencyPairs::get_currency_price(token, default).await {
                                    Ok(quote) => quote,
                                    Err(_)    => String::from("Uninitialized")
                                };
                            }

                            *current_price.lock().unwrap() = currency_quote;
                        }));
                    });
                }
                thread::sleep(Duration::from_secs(60));
            }
        });
    }

    pub fn get_exponent(exponent: i32) -> f64 {
        let digits: f64 = match exponent {
            19 => 1000000000000000000.0,
            18 => 100000000000000000.0,
            17 => 10000000000000000.0,
            16 => 1000000000000000.0,
            15 => 100000000000000.0,
            14 => 10000000000000.0,
            13 => 1000000000000.0,
            12 => 100000000000.0,
            11 => 10000000000.0,
            10 => 1000000000.0,
            9  => 100000000.0,
            8  => 10000000.0,
            7  => 1000000.0,
            6  => 100000.0,
            5  => 10000.0,
            4  => 1000.0,
            3  => 100.0,
            2  => 10.0,
            1  => 1.0,
            0  => 0.0,
            _  => 0.0
        };

        return digits;
    }
}