use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use gtk::{Orientation, Image, Align};
use gtk::prelude::*;
use glib::{clone, Continue, MainContext, PRIORITY_DEFAULT};

use crate::ApplicationSettings;
use crate::currencies::tokens::Token;

pub fn asset_view(app_settings: Arc<Mutex<ApplicationSettings>>) -> (gtk::Box, Arc<Mutex<ApplicationSettings>>) {
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
    scrollable_box.append(&*generate_currency_boxes(app_settings.clone()).lock().unwrap());

    let scrollable_container = gtk::ScrolledWindow::builder()
        .child(&scrollable_box)
        .name("assets_scrollable_container")
        .build();

    asset_box.append(&scrollable_container);
    return (asset_box, app_settings);
}

pub fn generate_currency_boxes(app_settings: Arc<Mutex<ApplicationSettings>>) -> Arc<Mutex<gtk::Box>> {
    let currencies_box = Arc::new(Mutex::new(gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(12)
        .margin_bottom(12)
        .build()));
    
    let no_assets_label = gtk::Label::builder()
        .label("No assets to display.")
        .margin_top(5)
        .margin_start(5)
        .css_name("label-currency_name")
        .visible(false)
        .build();
    
    currencies_box.lock().unwrap().append(&no_assets_label);
    let curr_box = currencies_box.lock().unwrap().clone();
    let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);

    thread::spawn(move || {
        let mut has_run = false;
        let mut currency_boxes = HashMap::<String, f64>::new();
        loop {
            if has_run == true {
                thread::sleep(Duration::from_secs(60));
            } else {
                has_run = true;
                thread::sleep(Duration::from_secs(5));
            }

            let btc_string = "Uninitialized".to_string();
            let eth_string = "Uninitialized".to_string();
            
            let app_settings = &*app_settings.lock().unwrap();
            let mut btc_balance = 0.0;
            let mut eth_balance = 0.0;

            for i in 0..app_settings.btc_wallets.len() {
                let btcw = app_settings.btc_wallets[i].clone();
                let balance = &*btcw.balance.lock().unwrap();
                btc_balance += balance.parse::<f64>().unwrap();
            }

            btc_balance = 15.3;
            if currency_boxes.contains_key("BTC") {
                if btc_balance > 0.0 {
                    currency_boxes.insert(String::from("BTC"), btc_balance);
                } else {
                    currency_boxes.insert(String::from("BTC"), 0.0);
                }
            } else if btc_balance > 0.0 {
                currency_boxes.insert(String::from("BTC"), btc_balance);
            }

            for i in 0..app_settings.eth_wallets.len() {
                let ethw = app_settings.eth_wallets[i].clone();
                let balance = &*ethw.balance.lock().unwrap();
                eth_balance += balance.parse::<f64>().unwrap();
            }

            if currency_boxes.contains_key("ETH") {
                if eth_balance > 0.0 {
                    currency_boxes.insert(String::from("ETH"), eth_balance);
                } else {
                    currency_boxes.insert(String::from("ETH"), 0.0);
                }
            } else if eth_balance > 0.0 {
                currency_boxes.insert(String::from("ETH"), eth_balance);
            }

            match sender.send(currency_boxes.clone()) {
                Ok(_) => {},
                Err(_) => {}
            };
        }
    });

    receiver.attach(
        None,
        clone!(@weak curr_box => @default-return Continue(false),
            move |currency_boxes| {
                println!("Received! length = {}", currency_boxes.len());
                //for (key, value) in currency_boxes {
                //    println!("Key: {}", key);
                //}
                Continue(true)
            }
        ),
    );

    return currencies_box;
}

pub fn generate_currency_box(token: Token, arc_label: Arc<Mutex<gtk::Label>>, arc_currency_box: Arc<Mutex<gtk::Box>>) {
    let label        = &*arc_label.lock().unwrap();
    let currency_box = &*arc_currency_box.lock().unwrap();
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

    icon_box.append(&icon);
    name_box.append(&currency_name);
    name_box.append(&currency_ticker);
    price_box.append(label);
    currency_box.append(&icon_box);
    currency_box.append(&name_box);
    currency_box.append(&price_box);
}