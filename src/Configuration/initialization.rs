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

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Clone)]
struct L3 {
    symbol       : String,
    name         : String,
    decimals     : i32,
    address      : String,
    logoURI      : Option<String>,
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

    let mut ipath = match ApplicationSettings::find_images_path(){
        Ok(mut ip) => {
            ip.push("Icons");
            ip
        },
        Err(_) => { 
            let mut path = ApplicationSettings::find_config_path().unwrap();
            path.pop();
            path.push("Imges/Icons");
            path
        }
    };

    let icons_path = ipath.display().to_string();
    fs::create_dir_all(ipath.clone())?;
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

    let logo = reqwest::get("https://github.com/BlockBreakersHQ/BlockWallet/raw/main/Images/Logo.png").await?;
    let logo_path = format!("{}/logo.png", ipath.pop().clone());
    let mut logo_out = File::create(logo_path.clone()).expect("failed to create file");
    logo_out.write_all(&mut logo.bytes().await?)?;

    let settings = reqwest::get("https://github.com/BlockBreakersHQ/BlockWallet/raw/main/Images/cog.png").await?;
    let settings_path = format!("{}/settings.png", ipath.pop().clone());
    let mut settings_out = File::create(settings_path.clone()).expect("failed to create file");
    settings_out.write_all(&mut settings.bytes().await?)?;

    let btc_icon = reqwest::get("https://dynamic-assets.coinbase.com/e785e0181f1a23a30d9476038d9be91e9f6c63959b538eabbc51a1abc8898940383291eede695c3b8dfaa1829a9b57f5a2d0a16b0523580346c6b8fab67af14b/asset_icons/b57ac673f06a4b0338a596817eb0a50ce16e2059f327dc117744449a47915cb2.png").await?;
    let btc_path = format!("{}/{}.png", icons_path, "BTC");
    let mut btc_out = File::create(btc_path.clone()).expect("failed to create file");
    btc_out.write_all(&mut btc_icon.bytes().await?)?;

    for (key, _value) in &currencies.address {
        let currency: &L3 = &currencies.address.get(key).unwrap();
        if !currency.symbol.contains("REALTOKEN") && !currency.symbol.contains("/") {
            let logoURI = match currency.logoURI.clone() {
                Some(l) => l,
                None => continue
            };
            let icon = reqwest::get(logoURI.clone()).await?;
            let icon_path = format!("{}/{}.png", icons_path, currency.symbol);
            let mut out = File::create(icon_path.clone()).expect("failed to create file");

            out.write_all(&mut icon.bytes().await?)?;
        }
    }
    println!("INFO: Downloading icons complete.");
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
        if !currency.symbol.contains("REALTOKEN") && !currency.symbol.contains("/") {
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