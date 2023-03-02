use adw::prelude::*;
use gtk::{Button, Orientation, Image};
use std::sync::{Arc, Mutex};

use crate::configuration::application_settings::*;
use crate::currencies::eth;
use crate::currencies::eth::EthereumWallet;
use crate::currencies::btc;
use crate::currencies::btc::BitcoinWallet;

pub fn wallet_view(app_settings: Arc<Mutex<ApplicationSettings>>) -> (gtk::Box, Arc<Mutex<ApplicationSettings>>) {
    let btc_data_displayed = Arc::new(Mutex::new(false));
    let eth_data_displayed = Arc::new(Mutex::new(false));

    let btc_wallets = app_settings.lock().unwrap().btc_wallets.clone();
    let eth_wallets = app_settings.lock().unwrap().eth_wallets.clone();

    let scrollable_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(12)
        .margin_bottom(12)
        .vexpand(true)
        .build();
    scrollable_box.set_widget_name("wallet_scrollable_box");

    let scrollable_container = gtk::ScrolledWindow::builder()
        .child(&scrollable_box)
        .name("wallet_scrollable_container")
        .build();

    let btc_button = Button::builder()
        .label("Bitcoin")
        .margin_top(6)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    btc_button.add_css_class("standard_button");

    let eth_button = Button::builder()
        .label("Ethereum")
        .margin_top(6)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    eth_button.add_css_class("standard_button");

    let add_wallet_button = Button::builder()
        .label("Add Wallet")
        .margin_top(6)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    add_wallet_button.add_css_class("standard_button");

    let wallet_box = Arc::new(Mutex::new(gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(12)
        .margin_bottom(12)
        .build()));
    
    scrollable_box.append(&btc_button);
    scrollable_box.append(&eth_button);
    scrollable_box.append(&add_wallet_button);

    let btc_currency_details = populate_btc_currency_details(&btc_wallets);
    let eth_currency_details = populate_eth_currency_details(&eth_wallets);
    scrollable_box.insert_child_after(&btc_currency_details, Some(&btc_button));
    scrollable_box.insert_child_after(&eth_currency_details, Some(&eth_button));
    let btc_currency_details_clone = btc_currency_details.clone();
    let eth_currency_details_clone = eth_currency_details.clone();

    let app_settings_clone = app_settings.clone();
    let new_wallet_box = new_wallet_box(app_settings_clone, Arc::new(Mutex::new(btc_currency_details.clone())),
        Arc::new(Mutex::new(eth_currency_details.clone())), Arc::new(Mutex::new(scrollable_container.clone())));
    wallet_box.lock().unwrap().append(&scrollable_container);
    wallet_box.lock().unwrap().append(&new_wallet_box);

    btc_button.connect_clicked(move |_button| {
        if *btc_data_displayed.lock().unwrap() == false {
            *btc_data_displayed.lock().unwrap() = true;
            btc_currency_details.set_visible(true);
            eth_currency_details_clone.set_visible(false);
        } else {
            *btc_data_displayed.lock().unwrap() = false;
            btc_currency_details.set_visible(false);
        }
    });

    eth_button.connect_clicked(move |_button| {
        if *eth_data_displayed.lock().unwrap() == false {
            *eth_data_displayed.lock().unwrap() = true;
            eth_currency_details.set_visible(true);
            btc_currency_details_clone.set_visible(false);
        } else {
            *eth_data_displayed.lock().unwrap() = false;
            eth_currency_details.set_visible(false);
        }
    });

    add_wallet_button.connect_clicked(move |_button| {
        new_wallet_box.set_visible(true);
        scrollable_container.set_visible(false);
    });

    return (wallet_box.lock().unwrap().clone(), app_settings);
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
            .css_name("btc_wallet_details")
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
            .css_name("btc_wallet_details")
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
            .css_name("btc_wallet_details")
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
            .css_name("btc_wallet_details")
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
            } else {
                qr_box.set_visible(false);
                button.set_label("Show QR Code");
            }
        });

        let expander = adw::ExpanderRow::builder()
            .title(&wallet_name)
            .margin_start(12)
            .margin_end(12)
            .height_request(40)
            .css_name("wallet_expander_row")
            .icon_name("btc")
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
            } else {
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
            .css_name("eth_wallet_details")
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
            .css_name("eth_wallet_details")
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
            .css_name("eth_wallet_details")
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
            .css_name("eth_wallet_details")
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
            } else {
                qr_box.set_visible(false);
                button.set_label("Show QR Code");
            }
        });

        let expander = adw::ExpanderRow::builder()
            .title(&wallet_name)
            .margin_start(12)
            .margin_end(12)
            .height_request(40)
            .css_name("wallet_expander_row")
            .icon_name("ETH")
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
            } else {
                wallet_box.set_visible(false);
                button.set_label("Show wallet details");
            }
        });
        
        widgets.append(&wallet_box_1);
        widgets.append(&wallet_button);
    }
    return widgets;
}

fn add_btc_wallet(btc_box: &mut gtk::Box, btcw: &BitcoinWallet) -> gtk::Box {
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
        .css_name("btc_wallet_details")
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
        .css_name("btc_wallet_details")
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
        .css_name("btc_wallet_details")
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
        .css_name("btc_wallet_details")
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
    
    let qr_box = btc_qr_box(&btcw);
    let qr_box_1 = qr_box.clone();

    qr_button.connect_clicked(move |button| {
        if qr_box.get_visible() == false {
            qr_box.set_visible(true);
            button.set_label("Hide QR Code");
        } else {
            qr_box.set_visible(false);
            button.set_label("Show QR Code");
        }
    });

    let expander = adw::ExpanderRow::builder()
        .title(&wallet_name)
        .margin_start(12)
        .margin_end(12)
        .height_request(40)
        .css_name("wallet_expander_row")
        .icon_name("btc")
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
        } else {
            wallet_box.set_visible(false);
            button.set_label("Show wallet details");
        }
    });
    
    btc_box.append(&wallet_box_1);
    btc_box.append(&wallet_button);
    return btc_box.clone();
}

fn add_eth_wallet(eth_box: &mut gtk::Box, ethw: &EthereumWallet) -> gtk::Box {
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
        .css_name("eth_wallet_details")
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
        .css_name("eth_wallet_details")
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
        .css_name("eth_wallet_details")
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
        .css_name("eth_wallet_details")
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
    
    let qr_box = eth_qr_box(&ethw);
    let qr_box_1 = qr_box.clone();

    qr_button.connect_clicked(move |button| {
        if qr_box.get_visible() == false {
            qr_box.set_visible(true);
            button.set_label("Hide QR Code");
        } else {
            qr_box.set_visible(false);
            button.set_label("Show QR Code");
        }
    });

    let expander = adw::ExpanderRow::builder()
        .title(&wallet_name)
        .margin_start(12)
        .margin_end(12)
        .height_request(40)
        .css_name("wallet_expander_row")
        .icon_name("ETH")
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
        } else {
            wallet_box.set_visible(false);
            button.set_label("Show wallet details");
        }
    });
    
    eth_box.append(&wallet_box_1);
    eth_box.append(&wallet_button);
    return eth_box.clone();
}

pub fn btc_qr_box(btcw: &BitcoinWallet) -> gtk::Box {
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

pub fn eth_qr_box(ethw: &EthereumWallet) -> gtk::Box {
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

fn new_wallet_box(app_settings: Arc<Mutex<ApplicationSettings>>, btc_box: Arc<Mutex<gtk::Box>>, 
    eth_box: Arc<Mutex<gtk::Box>>, scrollable_container: Arc<Mutex<gtk::ScrolledWindow>>) -> gtk::Box {
    let new_wallet_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .visible(false)
        .build();

    let tokens = vec!["Bitcoin", "Ethereum"];

    let token_selector = gtk::DropDown::from_strings(&tokens);
    token_selector.set_margin_start(12);
    token_selector.set_margin_end(12);
    token_selector.set_margin_top(12);
    token_selector.set_margin_bottom(12);

    let wallet_name = gtk::Entry::builder()
        .placeholder_text("Wallet Name")
        .margin_top(12)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .build();

    let create_wallet_button = Button::builder()
        .label("Create Wallet")
        .margin_top(6)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    create_wallet_button.add_css_class("standard_button");

    let mnemonic_error = gtk::Label::builder()
        .label("An error occurred when parsing the mnemonic. Please check the mnemonic and try again.")
        .margin_top(5)
        .margin_start(5)
        .visible(false)
        .css_name("label-error")
        .build();

    let wallet_generation_error = gtk::Label::builder()
        .label("An error occurred when generating the wallet. Please try again.")
        .margin_top(5)
        .margin_start(5)
        .visible(false)
        .css_name("label-error")
        .build();

    new_wallet_box.append(&token_selector);
    new_wallet_box.append(&wallet_name);
    new_wallet_box.append(&mnemonic_error);
    new_wallet_box.append(&wallet_generation_error);
    new_wallet_box.append(&create_wallet_button);

    let new_wallet_box_clone = new_wallet_box.clone();
    
    create_wallet_button.connect_clicked(move |_button| {
        let mut path = String::from("m/44'/60'/0'/0'/");
        if token_selector.selected() == 0 {
            path += &app_settings.lock().unwrap().btc_wallets.len().to_string();
            let mnemonic = match app_settings.lock().unwrap().btc_wallets[0].mnemonic.clone() {
                Some(mnemonic) => mnemonic,
                None           => {
                    mnemonic_error.set_visible(true);
                    return;
                }
            };
            let mut btcw = match btc::generate_from_mnemonic(&mnemonic, &path) {
                Some(btcw) => btcw,
                None       => {
                    wallet_generation_error.set_visible(true);
                    return;
                }
            };
            btcw.wallet_name = Some(wallet_name.text().to_string());
            let btcwc = btcw.clone();
            app_settings.lock().unwrap().btc_wallets.push(btcw);
            add_btc_wallet(&mut btc_box.lock().unwrap(), &btcwc);
            new_wallet_box.set_visible(false);
            scrollable_container.lock().unwrap().set_visible(true);
        } else if token_selector.selected() == 1 {
            path += &app_settings.lock().unwrap().eth_wallets.len().to_string();
            let mnemonic = match app_settings.lock().unwrap().eth_wallets[0].mnemonic.clone() {
                Some(mnemonic) => mnemonic,
                None           => {
                    mnemonic_error.set_visible(true);
                    return;
                }
            };
            let mut ethw = match eth::generate_from_mnemonic(&mnemonic, &path) {
                Some(ethw) => ethw,
                None       => {
                    wallet_generation_error.set_visible(true);
                    return;
                }
            };
            ethw.wallet_name = Some(wallet_name.text().to_string());
            let ethwc = ethw.clone();
            app_settings.lock().unwrap().eth_wallets.push(ethw);
            add_eth_wallet(&mut eth_box.lock().unwrap(), &ethwc);
            new_wallet_box.set_visible(false);
            scrollable_container.lock().unwrap().set_visible(true);
        }
    });

    return new_wallet_box_clone;
}