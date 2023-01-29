use adw::prelude::*;
use gtk::{Button, Orientation, Image};
use std::sync::{Arc, Mutex};

use crate::configuration::application_settings::*;
use crate::currencies::eth::EthereumWallet;
use crate::currencies::btc::BitcoinWallet;

pub fn wallet_view(app_settings: ApplicationSettings) -> (gtk::Box, ApplicationSettings) {
    let btc_data_displayed = Arc::new(Mutex::new(false));
    let eth_data_displayed = Arc::new(Mutex::new(false));

    let btc_wallets = app_settings.clone().btc_wallets;
    let eth_wallets = app_settings.clone().eth_wallets;

    let btc_button = Button::builder()
        .label("Bitcoin")
        .margin_top(6)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    
    btc_button.add_css_class("btc_button");

    let eth_button = Button::builder()
        .label("Ethereum")
        .margin_top(6)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    eth_button.add_css_class("eth_button");

    let asset_box = Arc::new(Mutex::new(gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(12)
        .margin_bottom(12)
        .build()));

    asset_box.lock().unwrap().append(&btc_button);
    asset_box.lock().unwrap().append(&eth_button);

    let btc_currency_details = populate_btc_currency_details(&btc_wallets);
    let eth_currency_details = populate_eth_currency_details(&eth_wallets);
    asset_box.lock().unwrap().insert_child_after(&btc_currency_details, Some(&btc_button));
    asset_box.lock().unwrap().insert_child_after(&eth_currency_details, Some(&eth_button));
    let btc_currency_details_clone = btc_currency_details.clone();
    let eth_currency_details_clone = eth_currency_details.clone();

    btc_button.connect_clicked(move |_button| {
        if *btc_data_displayed.lock().unwrap() == false {
            *btc_data_displayed.lock().unwrap() = true;
            btc_currency_details.set_visible(true);
            eth_currency_details_clone.set_visible(false);
        }
        else {
            *btc_data_displayed.lock().unwrap() = false;
            btc_currency_details.set_visible(false);
        }
    });

    eth_button.connect_clicked(move |_button| {
        if *eth_data_displayed.lock().unwrap() == false {
            *eth_data_displayed.lock().unwrap() = true;
            eth_currency_details.set_visible(true);
            btc_currency_details_clone.set_visible(false);
        }
        else {
            *eth_data_displayed.lock().unwrap() = false;
            eth_currency_details.set_visible(false);
        }
    });

    return (asset_box.lock().unwrap().clone(), app_settings);
}

fn populate_btc_currency_details(btc_wallets: &Vec<BitcoinWallet>) -> gtk::Box {
    let widgets = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .visible(false)
        .build();
    
    for btcw in btc_wallets {
        let wallet_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .visible(false)
            .build();
            
        let btc_mnemonic = match &btcw.mnemonic {
            Some(mnemonic) => format!("Mnemonic: {}", mnemonic),
            None => String::from("Uninitialized")
        };
        let btc_mnemonic_label = gtk::Label::builder()
            .label(&btc_mnemonic)
            .halign(gtk::Align::Start)
            .max_width_chars(50)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .selectable(true)
            .wrap(true)
            .wrap_mode(pango::WrapMode::Char)
            .build();

        let btc_address = match &btcw.address {
            Some(address) => format!("Address: {}", address),
            None => String::from("Uninitialized")
        };
        
        let btc_address_label = gtk::Label::builder()
            .label(&btc_address)
            .halign(gtk::Align::Start)
            .max_width_chars(50)
            .margin_start(12)
            .margin_end(12)
            .selectable(true)
            .wrap(true)
            .wrap_mode(pango::WrapMode::Char)
            .build();

        let btc_private_key = match &btcw.private_key {
            Some(private_key) => format!("Private_Key: {}", private_key),
            None => String::from("Uninitialized")
        };
        let btc_private_key_label = gtk::Label::builder()
            .label(&btc_private_key)
            .halign(gtk::Align::Start)
            .max_width_chars(50)
            .margin_start(12)
            .margin_end(12)
            .selectable(true)
            .wrap(true)
            .wrap_mode(pango::WrapMode::Char)
            .build();

        let btc_public_key = match &btcw.public_key {
            Some(public_key) => format!("Public_Key: {}", public_key),
            None => String::from("Uninitialized")
        };
        let btc_public_key_label = gtk::Label::builder()
            .label(&btc_public_key)
            .halign(gtk::Align::Start)
            .max_width_chars(50)
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(12)
            .selectable(true)
            .wrap(true)
            .wrap_mode(pango::WrapMode::Char)
            .build();

        let qr_button = Button::builder()
            .label("Show QR Code")
            .margin_top(6)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        let wallet_name = match &btcw.wallet_name {
            Some(wn) => wn,
            None     => "Unnamed Wallet"
        };

        let wallet_button = Button::builder()
            .label(&wallet_name)
            .margin_top(6)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();
        
        let qr_box = btc_qr_box(btcw);
        let qr_box_1 = qr_box.clone();

        qr_button.connect_clicked(move |button| {
            if qr_box.get_visible() == false {
                qr_box.set_visible(true);
                button.set_label("Hide QR Code");
            }
            else {
                qr_box.set_visible(false);
                button.set_label("Show QR Code");
            }
        });

        let expander = adw::ExpanderRow::builder()
            .title(&wallet_name)
            .margin_start(12)
            .margin_end(12)
            .height_request(40)
            .build();

        expander.add_row(&btc_mnemonic_label);
        expander.add_row(&btc_address_label);
        expander.add_row(&btc_private_key_label);
        expander.add_row(&btc_public_key_label);
        wallet_box.append(&expander);
        wallet_box.append(&qr_box_1);
        wallet_box.append(&qr_button);
        let wallet_box_1 = wallet_box.clone();

        wallet_button.connect_clicked(move |button| {
            if wallet_box.get_visible() == false {
                wallet_box.set_visible(true);
                button.set_label("Hide wallet details");
            }
            else {
                wallet_box.set_visible(false);
                button.set_label("Show wallet details");
            }
        });
        
        widgets.append(&wallet_box_1);
        widgets.append(&wallet_button);
    }
    return widgets;
}

fn populate_eth_currency_details(eth_wallets: &Vec<EthereumWallet>) -> gtk::Box {
    let widgets = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .visible(false)
        .build();

    for ethw in eth_wallets {
        let wallet_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .visible(false)
            .build();
            
        let eth_mnemonic = match &ethw.mnemonic {
            Some(mnemonic) => format!("Mnemonic: {}", mnemonic),
            None => String::from("Uninitialized")
        };
        let eth_mnemonic_label = gtk::Label::builder()
            .label(&eth_mnemonic)
            .halign(gtk::Align::Start)
            .max_width_chars(50)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .selectable(true)
            .wrap(true)
            .wrap_mode(pango::WrapMode::Char)
            .build();

        let eth_address = match &ethw.address {
            Some(address) => format!("Address: {}", address),
            None => String::from("Uninitialized")
        };
        
        let eth_address_label = gtk::Label::builder()
            .label(&eth_address)
            .halign(gtk::Align::Start)
            .max_width_chars(50)
            .margin_start(12)
            .margin_end(12)
            .selectable(true)
            .wrap(true)
            .wrap_mode(pango::WrapMode::Char)
            .build();

        let eth_private_key = match &ethw.private_key {
            Some(private_key) => format!("Private_Key: {}", private_key),
            None => String::from("Uninitialized")
        };
        let eth_private_key_label = gtk::Label::builder()
            .label(&eth_private_key)
            .halign(gtk::Align::Start)
            .max_width_chars(50)
            .margin_start(12)
            .margin_end(12)
            .selectable(true)
            .wrap(true)
            .wrap_mode(pango::WrapMode::Char)
            .build();

        let eth_public_key = match &ethw.public_key {
            Some(public_key) => format!("Public_Key: {}", public_key),
            None => String::from("Uninitialized")
        };
        let eth_public_key_label = gtk::Label::builder()
            .label(&eth_public_key)
            .halign(gtk::Align::Start)
            .max_width_chars(50)
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(12)
            .selectable(true)
            .wrap(true)
            .wrap_mode(pango::WrapMode::Char)
            .build();

        let qr_button = Button::builder()
            .label("Show QR Code")
            .margin_top(6)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        let wallet_name = match &ethw.wallet_name {
            Some(wn) => wn,
            None     => "Unnamed Wallet"
        };

        let wallet_button = Button::builder()
            .label(&wallet_name)
            .margin_top(6)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();
        
        let qr_box = eth_qr_box(ethw);
        let qr_box_1 = qr_box.clone();

        qr_button.connect_clicked(move |button| {
            if qr_box.get_visible() == false {
                qr_box.set_visible(true);
                button.set_label("Hide QR Code");
            }
            else {
                qr_box.set_visible(false);
                button.set_label("Show QR Code");
            }
        });

        let expander = adw::ExpanderRow::builder()
            .title(&wallet_name)
            .margin_start(12)
            .margin_end(12)
            .height_request(40)
            .build();

        expander.add_row(&eth_mnemonic_label);
        expander.add_row(&eth_address_label);
        expander.add_row(&eth_private_key_label);
        expander.add_row(&eth_public_key_label);
        wallet_box.append(&expander);
        wallet_box.append(&qr_box_1);
        wallet_box.append(&qr_button);
        let wallet_box_1 = wallet_box.clone();

        wallet_button.connect_clicked(move |button| {
            if wallet_box.get_visible() == false {
                wallet_box.set_visible(true);
                button.set_label("Hide wallet details");
            }
            else {
                wallet_box.set_visible(false);
                button.set_label("Show wallet details");
            }
        });
        
        widgets.append(&wallet_box_1);
        widgets.append(&wallet_button);
    }
    return widgets;
}

fn btc_qr_box(btcw: &BitcoinWallet) -> gtk::Box {
    let qr_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .visible(false)
            .build();

    let qr_code = btcw.generate_qr_address().unwrap();
    let qr_image = Image::from_paintable(Some(&qr_code));
    qr_image.set_pixel_size(300);

    qr_box.append(&qr_image);
    return qr_box;
}

fn eth_qr_box(ethw: &EthereumWallet) -> gtk::Box {
    let qr_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .visible(false)
            .build();

    let qr_code = ethw.generate_qr_address().unwrap();
    let qr_image = Image::from_paintable(Some(&qr_code));
    qr_image.set_pixel_size(300);

    qr_box.append(&qr_image);
    return qr_box;
}