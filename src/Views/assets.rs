use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};
use gtk::Orientation;
use gtk::prelude::*;
use glib::{clone, Continue, MainContext, PRIORITY_DEFAULT};
use crate::ApplicationSettings;

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

    let scrollable_container = gtk::ScrolledWindow::builder()
        .child(&scrollable_box)
        .name("assets_scrollable_container")
        .build();

    return (asset_box, app_settings);
}

/*
pub fn asset_view(app_settings: Arc<Mutex<ApplicationSettings>>) -> (gtk::Box, Arc<Mutex<ApplicationSettings>>) {
    let asset_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(12)
        .margin_bottom(12)
        .build();

    let btc_box = gtk::Box::new(Orientation::Horizontal, 185);

    let btc_label  = gtk::Label::builder()
        .label("BTC")
        .margin_top(12)
        .margin_start(50)
        .build();
    
    let btc_balance_label  = gtk::Label::builder()
        .label("Uninitialized")
        .margin_top(12)
        .margin_end(50)
        .build();

    btc_box.append(&btc_label);
    btc_box.append(&btc_balance_label);

    let eth_box = gtk::Box::new(Orientation::Horizontal, 185);
    let eth_label  = gtk::Label::builder()
        .label("ETH")
        .margin_top(12)
        .margin_start(50)
        .build();

    let eth_balance_label = gtk::Label::builder()
        .label("Uninitialized")
        .margin_top(12)
        .margin_end(50)
        .build();

    eth_box.append(&eth_label);
    eth_box.append(&eth_balance_label);

    asset_box.append(&btc_box);
    asset_box.append(&eth_box);

    let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);
    let app_settings_clone = app_settings.clone();

    thread::spawn(move || {
        loop {
            let sender  = sender.clone();

            let btc_balance = match &app_settings.lock().unwrap().btc_wallets[0].balance {
                Some(b) => b.clone(),
                None    => panic!("An error occurred in assets")
            };

            let eth_balance = match &app_settings.lock().unwrap().eth_wallets[0].balance {
                Some(b) => b.clone(),
                None    => panic!("An error occurred in assets")
            };

            let balance = (Arc::clone(&btc_balance), Arc::clone(&eth_balance));

            match sender.send(balance) {
                Ok(_) => {},
                Err(_) => {}
            };
            thread::sleep(Duration::from_secs(20));
        }
    });

    receiver.attach(
        None,
        clone!(@weak eth_balance_label => @default-return Continue(false),
            move |balance_text| {
                if *balance_text.0.lock().unwrap() != "Uninitialized" {
                    btc_balance_label.set_label(&*balance_text.0.lock().unwrap());
                }

                if *balance_text.1.lock().unwrap() != "Uninitialized" {
                    eth_balance_label.set_label(&*balance_text.1.lock().unwrap());
                }

                Continue(true)
            }
        ),
    );

    return (asset_box, app_settings_clone);
}
*/