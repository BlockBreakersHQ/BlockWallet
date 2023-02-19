use std::error::Error;
use std::collections::HashMap;
use std::fs::*;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use serde::Deserialize;

use crate::ApplicationSettings;
use crate::currencies::tokens::*;

#[derive(Debug, Deserialize)]
struct L1 {
    #[serde(flatten)]
    tokens: HashMap<String, L2>,
}

#[derive(Debug, Deserialize)]
struct L2 {
    #[serde(flatten)]
    address: HashMap<String, L3>,
}

#[derive(Debug, Deserialize, Clone)]
struct L3 {
    symbol       : String,
    name         : String,
    decimals     : i32,
    address      : String,
    logoURI      : String,
}

pub async fn download_icons() -> Result<String, Box<dyn Error>> {
    let resp = reqwest::get("https://api.1inch.io/v5.0/1/tokens")
        .await?
        .text()
        .await?;

    let cpath = match ApplicationSettings::find_config_path(){
        Ok(mut cp) => {
            cp.pop();
            cp.push("CurrencyDetails.json");
            cp
        },
        Err(_) => PathBuf::new()
    };

    let ipath = match ApplicationSettings::find_images_path(){
        Ok(mut ip) => {
            ip.push("Icons");
            ip
        },
        Err(_) => PathBuf::new()
    };

    fs::create_dir_all(ipath)?;
    if !cpath.exists() {
        match File::create(cpath.clone()) {
            Ok(mut cf) => {
                let _ = write!(cf, "{}", resp);
            },
            Err(e) => {
                ApplicationSettings::write_error_to_path(&ApplicationSettings::find_error_path()?, e.to_string());
            }
        };
    }

    let tokens_parsed: HashMap<String, L2> = serde_json::from_str(&resp)?;
    let currencies: &L2 = &tokens_parsed.get("tokens").unwrap();

    for (key, _value) in &currencies.address {
        let currency: &L3 = &currencies.address.get(key).unwrap();
        if !currency.symbol.contains("REALTOKEN") {
            let icon = reqwest::get(currency.logoURI.clone()).await?;
            let icon_path = format!("Icons/{}.png", currency.symbol);
            let mut out = File::create(icon_path.clone()).expect("failed to create file");

            out.write_all(&mut icon.bytes().await?)?;
        } else if currency.symbol.to_lowercase() == "btc"{
            let icon = reqwest::get("https://dynamic-assets.coinbase.com/e785e0181f1a23a30d9476038d9be91e9f6c63959b538eabbc51a1abc8898940383291eede695c3b8dfaa1829a9b57f5a2d0a16b0523580346c6b8fab67af14b/asset_icons/b57ac673f06a4b0338a596817eb0a50ce16e2059f327dc117744449a47915cb2.png").await?;
            let icon_path = format!("Icons/{}.png", currency.symbol);
            let mut out = File::create(icon_path.clone()).expect("failed to create file");

            out.write_all(&mut icon.bytes().await?)?;
        }
    }
    Ok(resp)
}

pub async fn download_token_details() -> Result<String, Box<dyn Error>> {
    let resp = reqwest::get("https://api.1inch.io/v5.0/1/tokens")
        .await?
        .text()
        .await?;

    let cpath = match ApplicationSettings::find_config_path(){
        Ok(mut cp) => {
            cp.pop();
            cp.push("CurrencyDetails.json");
            cp
        },
        Err(_) => PathBuf::new()
    };

    if !cpath.exists() {
        match File::create(cpath.clone()) {
            Ok(mut cf) => {
                let _ = write!(cf, "{}", resp);
            },
            Err(e) => {
                ApplicationSettings::write_error_to_path(&ApplicationSettings::find_error_path()?, e.to_string());
            }
        };
    }
    Ok(resp)
}

pub fn parse_token_details(currency_json: &str, mut tokens: Tokens) -> Result<Tokens, Box<dyn Error>> {
    let tokens_parsed: HashMap<String, L2> = serde_json::from_str(&currency_json)?;
    let contracts: &L2 = &tokens_parsed.get("tokens").unwrap();

    let mut icon_path = match ApplicationSettings::find_images_path(){
        Ok(mut ip) => {
            ip.push("Icons");
            ip
        },
        Err(_) => PathBuf::new()
    };

    let mut usable_keys: HashMap<String, L3> = HashMap::new();

    for (key, value) in &contracts.address {
        let currency: &L3 = &contracts.address.get(key).unwrap();
        if !currency.symbol.contains("REALTOKEN") {
            usable_keys.insert(String::from(key), value.clone());
        }
    }

    for (key, _value) in &usable_keys {
        icon_path.push(format!("{}.png", usable_keys.get(key).unwrap().symbol));
        tokens.eth_tokens.insert(usable_keys.get(key).unwrap().symbol.clone(),
            Token {
                symbol:     usable_keys.get(key).unwrap().symbol.clone(),
                name:       usable_keys.get(key).unwrap().name.clone(),
                decimals:   usable_keys.get(key).unwrap().decimals,
                address:    usable_keys.get(key).unwrap().address.clone(),
                logo:       icon_path.clone()
            },
        );
        icon_path.pop();
    }

    Ok(tokens)
}