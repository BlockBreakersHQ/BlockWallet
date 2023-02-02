use gtk::prelude::*;
use gtk::{CssProvider, StyleContext};
use gtk::gdk::{Display};
use adw::{Application, ApplicationWindow};
use std::path::{Path, PathBuf};
use std::thread;
use std::fs;

mod views;
mod currencies;
mod configuration;
mod tests;

use crate::views::{login};
use crate::configuration::initialization;
use crate::configuration::application_settings::*;

const APP_ID: &str = "org.BlockBreakers.Wallet";


fn main() {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| load_css());
    app.connect_activate(build_ui);
    app.run();
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
    } else if !Path::new(&currency_path).exists() {
        thread::spawn(move || {
            let _ = runtime.block_on(runtime.spawn(async move {
                match initialization::download_token_details().await {
                    Ok(_) => (),
                    Err(e) => {
                        ApplicationSettings::write_error_to_path(&ApplicationSettings::find_error_path().unwrap(), e.to_string());
                    }
                };
            }));
        });
    }

    let mut currencies = currencies::tokens::Tokens::new();
    let json = fs::read_to_string(currency_path).expect("Unable to read file");
    currencies = match initialization::parse_token_details(&json, currencies.clone()) {
        Ok(c) => c,
        Err(e) => {
            ApplicationSettings::write_error_to_path(&ApplicationSettings::find_error_path().unwrap(), e.to_string());
            currencies
        }
    };

    let window = ApplicationWindow::builder()
        .application(app)
        .title("BlockWallet")
        .default_width(360)
        .default_height(720)
        .build();

    let window_clone = window.clone();
    let app_settings = ApplicationSettings::new();
    login::login_view(window_clone, app_settings);
}