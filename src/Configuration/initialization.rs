use std::error::Error;
use std::collections::HashMap;
use std::fs::*;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use serde::Deserialize;
use glib::prelude::*;

use crate::ApplicationSettings;
use crate::currencies::tokens::*;

#[allow(dead_code)]
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

    let cpath = match ApplicationSettings::find_currency_details_path() {
        Ok(cp) => cp,
        Err(_) => PathBuf::new()
    };

    let ipath = match crate::configuration::paths::icon_cache_path() {
        Ok(ip) => ip,
        Err(_) => PathBuf::new()
    };

    let icons_path = ipath.display().to_string();
    fs::create_dir_all(ipath.clone())?;
    if !cpath.exists() {
        match File::create(cpath.clone()) {
            Ok(mut cf) => {
                let _ = write!(cf, "{}", resp);
            },
            Err(_) => {
                crate::configuration::logging::error("failed to write currency details cache");
            }
        };
    }

    let tokens_parsed: HashMap<String, L2> = serde_json::from_str(&resp)?;
    let currencies: &L2 = &tokens_parsed.get("tokens").unwrap();

    let images = crate::configuration::paths::images_path().unwrap_or_else(|_| PathBuf::new());
    let logo_path = images.join("Logo.png");
    if !logo_path.exists() && !images.as_os_str().is_empty() {
        let logo = reqwest::get("https://github.com/BlockBreakersHQ/BlockWallet/raw/main/Images/Logo.png").await?;
        let mut logo_out = File::create(&logo_path)?;
        logo_out.write_all(&mut logo.bytes().await?)?;
    }

    let settings_path = images.join("cog.png");
    if !settings_path.exists() && !images.as_os_str().is_empty() {
        let settings = reqwest::get("https://github.com/BlockBreakersHQ/BlockWallet/raw/main/Images/cog.png").await?;
        let mut settings_out = File::create(&settings_path)?;
        settings_out.write_all(&mut settings.bytes().await?)?;
    }

    let btc_icon = reqwest::get("https://dynamic-assets.coinbase.com/e785e0181f1a23a30d9476038d9be91e9f6c63959b538eabbc51a1abc8898940383291eede695c3b8dfaa1829a9b57f5a2d0a16b0523580346c6b8fab67af14b/asset_icons/b57ac673f06a4b0338a596817eb0a50ce16e2059f327dc117744449a47915cb2.png").await?;
    let btc_path = format!("{}/{}.png", icons_path, "BTC");
    let mut btc_out = File::create(btc_path.clone()).expect("failed to create file");
    btc_out.write_all(&mut btc_icon.bytes().await?)?;

    for (key, _value) in &currencies.address {
        let currency: &L3 = &currencies.address.get(key).unwrap();
        if !currency.symbol.contains("REALTOKEN") && !currency.symbol.contains("/") {
            let logo_uri = match currency.logoURI.clone() {
                Some(l) => l,
                None => continue
            };
            let icon = reqwest::get(logo_uri.clone()).await?;
            let icon_path = format!("{}/{}.png", icons_path, currency.symbol);
            let mut out = File::create(icon_path.clone()).expect("failed to create file");

            out.write_all(&mut icon.bytes().await?)?;
        }
    }
    tracing::info!("token icon download complete");
    Ok(resp)
}

pub async fn download_token_details() -> Result<String, Box<dyn Error>> {
    let resp = reqwest::get("https://api.1inch.io/v5.0/1/tokens")
        .await?
        .text()
        .await?;

    let cpath = match ApplicationSettings::find_currency_details_path() {
        Ok(cp) => cp,
        Err(_) => PathBuf::new()
    };

    if !cpath.exists() {
        match File::create(cpath.clone()) {
            Ok(mut cf) => {
                let _ = write!(cf, "{}", resp);
            },
            Err(_) => {
                crate::configuration::logging::error("failed to write currency details cache");
            }
        };
    }
    Ok(resp)
}

pub fn load_tokens() -> Tokens {
    let tokens = Tokens::new();
    let path = match ApplicationSettings::find_currency_details_path() {
        Ok(path) => path,
        Err(_) => return tokens,
    };
    let json = match fs::read_to_string(&path) {
        Ok(json) if !json.is_empty() => json,
        _ => return tokens,
    };
    match parse_token_details(&json, tokens.clone()) {
        Ok(parsed) => parsed,
        Err(_) => {
            crate::configuration::logging::error("failed to parse token details cache");
            tokens
        }
    }
}

pub fn parse_token_details(currency_json: &str, mut tokens: Tokens) -> Result<Tokens, Box<dyn Error>> {
    let tokens_parsed: HashMap<String, L2> = serde_json::from_str(&currency_json)?;
    let contracts: &L2 = &tokens_parsed.get("tokens").unwrap();

    let mut usable_keys: HashMap<String, L3> = HashMap::new();

    for (key, value) in &contracts.address {
        let currency: &L3 = &contracts.address.get(key).unwrap();
        if !currency.symbol.contains("REALTOKEN") && !currency.symbol.contains("/") {
            usable_keys.insert(String::from(key), value.clone());
        }
    }

    let token_list = gio::ListStore::new::<glib::BoxedAnyObject>();

    for (key, _value) in &usable_keys {
        let logo = crate::configuration::paths::token_icon_path(&usable_keys.get(key).unwrap().symbol);
        token_list.append(&glib::BoxedAnyObject::new(Token {
            symbol:     usable_keys.get(key).unwrap().symbol.clone(),
            name:       usable_keys.get(key).unwrap().name.clone(),
            decimals:   usable_keys.get(key).unwrap().decimals,
            address:    usable_keys.get(key).unwrap().address.clone(),
            logo:       logo.clone(),
            chain:      String::from("eth"),
        }));

        tokens.eth_tokens.insert(format!("eth:{}", usable_keys.get(key).unwrap().symbol),
            Token {
                symbol:     usable_keys.get(key).unwrap().symbol.clone(),
                name:       usable_keys.get(key).unwrap().name.clone(),
                decimals:   usable_keys.get(key).unwrap().decimals,
                address:    usable_keys.get(key).unwrap().address.clone(),
                logo,
                chain:      String::from("eth"),
            },
        );
    }
    
    Ok(tokens)
}