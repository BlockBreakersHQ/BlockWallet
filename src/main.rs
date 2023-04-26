use gtk::prelude::*;
use gtk::{CssProvider, StyleContext};
use gtk::gdk::Display;
use adw::{Application, ApplicationWindow};
use std::path::{Path, PathBuf};
use std::thread;
use std::fs;
use std::time::Duration;

mod views;
mod currencies;
mod configuration;
mod tests;

use crate::views::{login};
use crate::configuration::initialization;
use crate::configuration::application_settings::*;

use crate::currencies::tokens::Token;

const APP_ID: &str = "org.BlockBreakers.Wallet";

fn main() {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| load_css());
    app.connect_startup(|_| load_images());
    app.connect_activate(build_ui);
    app.run();
}

fn load_images() {
    let icon_path = match ApplicationSettings::find_images_path(){
        Ok(mut lp) => {
            lp.push("Icons");
            lp
        },
        Err(_) => PathBuf::new()
    };          

    let icons = gtk::IconTheme::for_display(&Display::default().expect("Could not connect to a display."));
    icons.add_search_path(icon_path);
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_data(include_bytes!("style.css"));

    StyleContext::add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

pub fn build_ui(app: &Application) {
    let icon_path = match ApplicationSettings::find_images_path(){
        Ok(mut lp) => {
            lp.push("Icons");
            lp
        },
        Err(_) => PathBuf::new()
    };

    let currency_path = match ApplicationSettings::find_config_path(){
        Ok(mut cp) => {
            cp.pop();
            cp.push("CurrencyDetails.json");
            cp
        },
        Err(_) => PathBuf::new()
    };

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let runtime2 = tokio::runtime::Runtime::new().unwrap();

    if !Path::new(&icon_path).is_dir() {
        thread::spawn(move || {
            let _ = runtime.block_on(runtime.spawn(async move {
                match initialization::download_icons().await {
                    Ok(_) => (),
                    Err(e) => {
                        ApplicationSettings::write_error_to_path(&ApplicationSettings::find_error_path().unwrap(), e.to_string());
                    }
                };
            }));
        });
    }
    if !Path::new(&currency_path).exists() {
        thread::spawn(move || {
            let _ = runtime2.block_on(runtime2.spawn(async move {
                match initialization::download_token_details().await {
                    Ok(_) => (),
                    Err(e) => {
                        ApplicationSettings::write_error_to_path(&ApplicationSettings::find_error_path().unwrap(), e.to_string());
                    }
                };
            }));
        });
    }

    let mut tokens = currencies::tokens::Tokens::new();
    let mut json = String::new();

    if !Path::new(&currency_path).exists() {
        thread::sleep(Duration::from_secs(3));
        json = fs::read_to_string(currency_path).expect("Unable to read file");
    } else {
        json = fs::read_to_string(currency_path).expect("Unable to read file");
    }
    
    tokens = match initialization::parse_token_details(&json, tokens.clone()) {
        Ok(c) => c,
        Err(e) => {
            println!("Error: {}", e);
            ApplicationSettings::write_error_to_path(&ApplicationSettings::find_error_path().unwrap(), e.to_string());
            tokens
        }
    };

    let window = ApplicationWindow::builder()
        .application(app)
        .title("BlockWallet")
        .default_width(360)
        .default_height(720)
        .build();

    let window_clone = window.clone();
    let app_settings = ApplicationSettings::new(tokens);
    login::login_view(window_clone, app_settings);
}