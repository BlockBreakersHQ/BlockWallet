use crate::Token;
use serde_json::Value;

use crate::ApplicationSettings;
use crate::currencies::currency_pairs::CurrencyPairs;

#[derive(Clone, Debug)]
pub struct Trade {
    pub from_token:     Token,
    pub to_token:       Token,
    pub from_amount:    f64,
    pub to_amount:      f64,
    pub estimated_gas:  f64
}

impl Trade {
    pub fn new(app_settings: ApplicationSettings) -> Trade {
        let eth = app_settings
            .tokens
            .eth_tokens
            .get("eth:ETH")
            .cloned()
            .unwrap_or_else(|| Token {
                name: String::from("Ethereum"),
                symbol: String::from("ETH"),
                address: String::from("0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"),
                logo: crate::configuration::paths::token_icon_path("ETH"),
                decimals: 18,
                chain: String::from("eth"),
            });
        let usdc = app_settings
            .tokens
            .eth_tokens
            .get("eth:USDC")
            .cloned()
            .unwrap_or_else(|| Token {
                name: String::from("USD Coin"),
                symbol: String::from("USDC"),
                address: String::from("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"),
                logo: crate::configuration::paths::token_icon_path("USDC"),
                decimals: 6,
                chain: String::from("eth"),
            });

        Trade {
            from_token: usdc,
            to_token: eth,
            from_amount: 0.0,
            to_amount: 0.0,
            estimated_gas: 0.0,
        }
    }

    //pub async fn get_quote(mut self, from: Token, to: Token, amount: f64)  {
    pub async fn get_quote(mut self) -> Trade {
        let amount = &self.from_amount * CurrencyPairs::get_exponent(self.from_token.decimals.clone());
        let from   = &self.from_token.address;
        let to     = &self.to_token.address;

        let one_inch_get_quote_url = 
            format!("https://api.1inch.io/v5.0/1/quote?\
            fromTokenAddress={}\
            &toTokenAddress={}\
            &amount={}", from, to, amount.to_string());

        let resp = match reqwest::get(one_inch_get_quote_url).await {
            Ok(resp) => resp,
            Err(_) => {
                crate::configuration::logging::warn("swap quote request failed");
                return self.clone();
            }
        };

        let text = match resp.text().await {
            Ok(text) => text,
            Err(_) => {
                crate::configuration::logging::warn("swap quote body read failed");
                return self.clone();
            }
        };

        let json: Value = match serde_json::from_str(&text) {
            Ok(r)  => r,
            Err(_) => {
                crate::configuration::logging::warn("swap quote parse failed");
                return self.clone();
            }
        };

        let Ok(mut to_amount) = json["toTokenAmount"].to_string().replace('"', "").parse::<f64>() else {
            return self.clone();
        };
        let Ok(to_decimals) = json["toToken"]["decimals"].to_string().parse::<i32>() else {
            return self.clone();
        };
        to_amount /= CurrencyPairs::get_exponent(to_decimals);
        let Ok(mut from_amount) = json["fromTokenAmount"].to_string().replace('"', "").parse::<f64>() else {
            return self.clone();
        };
        let Ok(from_decimals) = json["fromToken"]["decimals"].to_string().parse::<i32>() else {
            return self.clone();
        };
        from_amount /= CurrencyPairs::get_exponent(from_decimals);
        let Ok(estimated_gas) = json["estimatedGas"].to_string().replace('"', "").parse::<f64>() else {
            return self.clone();
        };


        self.estimated_gas = estimated_gas;
        self.from_amount = from_amount;
        self.to_amount = to_amount;
        self
    }
}