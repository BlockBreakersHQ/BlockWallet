use crate::Token;
use serde_json::Value;

use crate::ApplicationSettings;
use crate::currencies::currency_pairs::CurrencyPairs;

#[derive(Clone, Debug)]
pub struct Trade {
    from_token:     Token,
    to_token:       Token,
    from_amount:    f64,
    to_amount:      f64,
    estimated_gas:  f64
}

impl Trade {
    pub fn new(app_settings: ApplicationSettings) -> Trade {
        let eth  = app_settings.tokens.eth_tokens["ETH"].clone();
        let usdc = app_settings.tokens.eth_tokens["USDC"].clone();
        
        Trade {
            from_token:     eth,
            to_token:       usdc,
            from_amount:    0.0,
            to_amount:      0.0,
            estimated_gas:  0.0
        }
    }

    pub async fn get_quote(mut self, from: Token, to: Token, amount: f64)  {
        let amount = amount * CurrencyPairs::get_exponent(from.decimals);
        let from   = from.address;
        let to     = to.address;

        let one_inch_get_quote_url = 
            format!("https://api.1inch.io/v5.0/1/quote?\
            fromTokenAddress={}\
            &toTokenAddress={}\
            &amount={}", from, to, amount.to_string());

        let resp = match reqwest::get(one_inch_get_quote_url).await {
            Ok(resp) => resp,
            Err(e) => {println!("error: {}", e.to_string());return;}
        };

        let text = match resp.text().await {
            Ok(text) => text,
            Err(e) => {println!("error: {}", e.to_string());return;}
        };

        let json: Value = match serde_json::from_str(&text) {
            Ok(r)  => r,
            Err(e) => {println!("error: {}", e.to_string());return;}
        };

        let mut to_amount: f64 = json["toTokenAmount"].to_string().replace("\"", "").parse::<f64>().expect("ERROR: Parsing value failed.");
        to_amount = to_amount / CurrencyPairs::get_exponent(json["toToken"]["decimals"].to_string().parse::<i32>().expect("ERROR: Parsing value failed."));
        let mut from_amount: f64 = json["fromTokenAmount"].to_string().replace("\"", "").parse::<f64>().expect("ERROR: Parsing value failed.");
        from_amount = from_amount / CurrencyPairs::get_exponent(json["fromToken"]["decimals"].to_string().parse::<i32>().expect("ERROR: Parsing value failed."));
        let estimated_gas = json["estimatedGas"].to_string().replace("\"", "").parse::<f64>().expect("ERROR: Parsing value failed.");

        self.estimated_gas = estimated_gas;
        self.from_amount = from_amount;
        self.to_amount = to_amount;
    }
}