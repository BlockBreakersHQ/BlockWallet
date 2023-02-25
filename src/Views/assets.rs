use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use gtk::{Orientation, Image, Align};
use gtk::prelude::*;
use glib::{clone, Continue, MainContext, PRIORITY_DEFAULT};

use crate::ApplicationSettings;
use crate::currencies::tokens::Token;
use crate::views::currency::*;

pub fn asset_view(app_settings: Arc<Mutex<ApplicationSettings>>) -> (gtk::Box, Arc<Mutex<ApplicationSettings>>) {
    let app_settings_clone = app_settings.clone();
    let asset_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(12)
        .margin_bottom(12)
        .build();

    let scrollable_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(12)
        .margin_bottom(12)
        .vexpand(true)
        .build();
    scrollable_box.set_widget_name("assets_scrollable_box");

    let currencies_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(12)
        .margin_bottom(12)
        .build();
    currencies_box.set_widget_name("currencies_box");

    let no_assets_label = gtk::Label::builder()
        .label("No assets to display.")
        .margin_top(5)
        .margin_start(5)
        .css_name("label-currency_name")
        .visible(false)
        .build();
    no_assets_label.set_widget_name("no_assets_label");

    currencies_box.append(&no_assets_label);
    let curr_box = currencies_box.clone();
    let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);

    thread::spawn(move || {
        let mut has_run = false;
        let mut currency_boxes = HashMap::<String, (f64, Token)>::new();
        loop {
            if has_run == true {
                thread::sleep(Duration::from_secs(60));
            } else {
                has_run = true;
                thread::sleep(Duration::from_secs(5));
            }
            
            let app_settings = &app_settings.lock().unwrap().clone();
            let mut btc_balance = 0.0;
            let mut eth_balance = 0.0;

            for i in 0..app_settings.btc_wallets.len() {
                let btcw = app_settings.btc_wallets[i].clone();
                let balance = &*btcw.balance.lock().unwrap();
                btc_balance += match balance.parse::<f64>() {
                    Ok(b) => b,
                    Err(_) => 0.0
                };
            }
            
            if currency_boxes.contains_key("BTC") {
                if btc_balance > 0.0 {
                    currency_boxes.insert(String::from("BTC"), (btc_balance, app_settings.tokens.eth_tokens["BTC"].clone()));
                } else {
                    currency_boxes.insert(String::from("BTC"), (0.0, app_settings.tokens.eth_tokens["BTC"].clone()));
                }
            } else if btc_balance > 0.0 {
                currency_boxes.insert(String::from("BTC"), (btc_balance, app_settings.tokens.eth_tokens["BTC"].clone()));
            }

            for i in 0..app_settings.eth_wallets.len() {
                let ethw = app_settings.eth_wallets[i].clone();
                let balance = &*ethw.balance.lock().unwrap();
                let erc20_balances = &*ethw.erc20_balances.lock().unwrap();
                for (key, value) in erc20_balances {
                    if currency_boxes.contains_key(key) {
                        let (balance, token) = &currency_boxes[key];
                        currency_boxes.insert(key.clone(), (balance + value, token.clone()));
                    } else {
                        currency_boxes.insert(key.clone(), (value.clone(), app_settings.tokens.eth_tokens[key].clone()));
                    }
                }
                eth_balance += match balance.parse::<f64>() {
                    Ok(b) => b,
                    Err(_) => 0.0
                };
            }

            if currency_boxes.contains_key("ETH") {
                if eth_balance > 0.0 {
                    currency_boxes.insert(String::from("ETH"), (eth_balance, app_settings.tokens.eth_tokens["ETH"].clone()));
                } else {
                    currency_boxes.insert(String::from("ETH"), (0.0, app_settings.tokens.eth_tokens["ETH"].clone()));
                }
            } else if eth_balance > 0.0 {
                currency_boxes.insert(String::from("ETH"), (eth_balance, app_settings.tokens.eth_tokens["ETH"].clone()));
            }

            for (key, value) in currency_boxes.clone() {
                println!("key = {} value = {}", key, value.0);
            }
            
            match sender.send((currency_boxes.clone(), app_settings.clone())) {
                Ok(_) => {},
                Err(_) => {}
            };
        }
    });

    receiver.attach(
        None,
        clone!(@weak curr_box => @default-return Continue(false),
            move |currency_boxes| {
                let app_settings = currency_boxes.1;
                let currency_boxes = currency_boxes.0;
                
                let asett = app_settings.clone();
                let mut next_child = match curr_box.first_child() {
                    Some(c) => c,
                    None => return Continue(true)
                };
                let last_child = match curr_box.last_child() {
                    Some(c) => c,
                    None => return Continue(true)
                };

                let mut build_without_display = false;
                let mut old_currecny_detail = next_child.clone();
                while next_child != last_child {
                    if next_child.widget_name() == "currencies" {
                        let current_child = next_child.clone();
                        next_child = match next_child.next_sibling() {
                            Some(c) => c,
                            None => return Continue(true)
                        };
                        curr_box.remove(&current_child);
                        continue;
                    } else if next_child.widget_name() == "currency_detail_view" {
                        println!("Currency detail view found.");
                        old_currecny_detail = next_child.clone();
                        let current_child = next_child.clone();
                        if next_child.get_visible() == true {
                            build_without_display = true;
                        } else {
                            curr_box.remove(&current_child);
                        }
                    }
                    next_child = match next_child.next_sibling() {
                        Some(c) => c,
                        None => return Continue(true)
                    };
                }

                if last_child.widget_name() != "no_assets_label" && last_child.widget_name() != "currency_detail_view"{
                    curr_box.remove(&last_child);
                }

                let currencies = gtk::Box::builder()
                    .orientation(Orientation::Vertical)
                    .visible(true)
                    .build();
                currencies.set_widget_name("currencies");

                if build_without_display == true {
                    currencies.set_visible(false);
                }

                let currency_detail_view = gtk::Box::builder()
                    .orientation(Orientation::Vertical)
                    .margin_top(12)
                    .margin_bottom(12)
                    .visible(false)
                    .build();
                currency_detail_view.set_widget_name("currency_detail_view");

                if currency_boxes.len() > 0 {
                    curr_box.first_child().unwrap().set_visible(false);
                    for (key, value) in currency_boxes {
                        let currency_box = generate_currency_box(value.0, value.1.clone());
                        let currency_detail_clone = currency_detail_view.clone();
                        let gesture = gtk::GestureClick::new();
                        let token = value.1.clone();
                        let currencies_clone = currencies.clone();
                        let app_settings = app_settings.clone();
                        gesture.connect_released(move |gesture, _, _, _| {
                            gesture.set_state(gtk::EventSequenceState::Claimed);
                            if !currency_detail_clone.first_child().is_none() {
                                currency_detail_clone.remove(&currency_detail_clone.first_child().unwrap());
                            };
                            currency_detail_clone.append(&currency_view(token.clone(), app_settings.clone()));
                            currency_detail_clone.set_visible(true);
                            currencies_clone.set_visible(false);
                        });
                        currency_box.add_controller(&gesture);
                        currencies.append(&currency_box);
                    }
                    curr_box.append(&currency_detail_view);
                    if old_currecny_detail != last_child && build_without_display == false {
                        curr_box.remove(&old_currecny_detail);
                    }
                    curr_box.append(&currencies);
                } else {
                    curr_box.first_child().unwrap().set_visible(true);
                }

                Continue(true)
            }
        ),
    );

    scrollable_box.append(&currencies_box);

    let scrollable_container = gtk::ScrolledWindow::builder()
        .child(&scrollable_box)
        .name("assets_scrollable_container")
        .build();

    asset_box.append(&scrollable_container);
    return (asset_box, app_settings_clone);
}

pub fn generate_currency_box(balance: f64, token: Token) -> gtk::Box {
    let currency_box = gtk::Box::new(Orientation::Horizontal, 5);
    let icon_box     = gtk::Box::new(Orientation::Vertical, 0);
    let name_box     = gtk::Box::new(Orientation::Vertical, 0);
    let price_box    = gtk::Box::new(Orientation::Vertical, 0);

    let icon = Image::from_file(token.logo);
    
    icon.set_pixel_size(50);
    icon.set_margin_start(12);
    icon.set_margin_bottom(8);

    let currency_name  = gtk::Label::builder()
        .label(&token.name)
        .margin_top(5)
        .margin_start(5)
        .halign(Align::Start)
        .css_name("label-currency_name")
        .build();

    let currency_ticker  = gtk::Label::builder()
        .label(&token.symbol)
        .margin_top(5)
        .margin_start(5)
        .halign(Align::Start)
        .css_name("label-currency_ticker")
        .build();

    let currency_price_label  = gtk::Label::builder()
        .label(&format!("{:.5}", balance.to_string()))
        .margin_top(5)
        .margin_end(12)
        .halign(Align::End)
        .hexpand(true)
        .css_name("label-currency_price")
        .build();

    icon_box.append(&icon);
    name_box.append(&currency_name);
    name_box.append(&currency_ticker);
    price_box.append(&currency_price_label);
    currency_box.append(&icon_box);
    currency_box.append(&name_box);
    currency_box.append(&price_box);
    currency_box.set_widget_name(&token.symbol);
    
    return currency_box;
}