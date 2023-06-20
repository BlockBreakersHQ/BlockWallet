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
        let eth  = app_settings.tokens.eth_tokens["ETH"].clone();
        let usdc = app_settings.tokens.eth_tokens["USDC"].clone();
        
        Trade {
            from_token:     usdc,
            to_token:       eth,
            from_amount:    0.0,
            to_amount:      0.0,
            estimated_gas:  0.0
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
            Err(e) => {println!("error: {}", e.to_string());return self.clone();}
        };

        let text = match resp.text().await {
            Ok(text) => text,
            Err(e) => {println!("error: {}", e.to_string());return self.clone();}
        };

        let json: Value = match serde_json::from_str(&text) {
            Ok(r)  => r,
            Err(e) => {println!("error: {}", e.to_string());return self.clone();}
        };

        let mut to_amount: f64 = json["toTokenAmount"].to_string().replace("\"", "").parse::<f64>().expect("ERROR: Parsing value failed.");
        to_amount = to_amount / CurrencyPairs::get_exponent(json["toToken"]["decimals"].to_string().parse::<i32>().expect("ERROR: Parsing value failed."));
        let mut from_amount: f64 = json["fromTokenAmount"].to_string().replace("\"", "").parse::<f64>().expect("ERROR: Parsing value failed.");
        from_amount = from_amount / CurrencyPairs::get_exponent(json["fromToken"]["decimals"].to_string().parse::<i32>().expect("ERROR: Parsing value failed."));
        let estimated_gas = json["estimatedGas"].to_string().replace("\"", "").parse::<f64>().expect("ERROR: Parsing value failed.");


        self.estimated_gas = estimated_gas;
        self.from_amount = from_amount;
        self.to_amount = to_amount;
        self
    }
}