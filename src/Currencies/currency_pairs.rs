use serde_json::Value;
use crate::configuration::*;

#[derive(Clone, Debug)]
pub struct CurrencyPairs {
    pub default_currency    : Option<StableCoin>,
    pub btc_usd             : Option<String>,
    pub eth_usd             : Option<String>
}

impl CurrencyPairs {
    pub fn new() -> CurrencyPairs {
        CurrencyPairs {
            default_currency    : Some(StableCoin::USDC),
            btc_usd             : Some(String::from("0")),
            eth_usd             : Some(String::from("0"))
        }
    }
    
    pub async fn set_btc_price(&mut self) -> Result<(), reqwest::Error> {
        let resp = match reqwest::get("https://api.kine.exchange/market/api/price/BTCUSD").await?.text().await {
            Ok(r)  => r,
            Err(_) => return Ok(())
        };
        
        let json: Value = match serde_json::from_str(&resp) {
            Ok(r)  => r,
            Err(_) => return Ok(())
        };

        self.btc_usd = Some(json["data"]["price"].to_string());
        Ok(())
    }

    pub async fn set_eth_price(&mut self) -> Result<(), block_error::Error> {
        let resp = match reqwest::get("https://api.kine.exchange/market/api/price/ETHUSD").await?.text().await {
            Ok(r)  => r,
            Err(_) => return Ok(())
        };

        let json: Value = match serde_json::from_str(&resp) {
            Ok(r)  => r,
            Err(_) => return Ok(())
        };

        self.eth_usd = Some(json["data"]["price"].to_string());
        Ok(())
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

    pub async fn get_eth_price() -> Result<String, block_error::Error> {
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
}

#[derive(Clone, Copy, Debug)]
pub enum StableCoin {
    USDC,
    //BUSD,
    //DAI,
    //FEI,
    //FRAX,
    //USDT
}