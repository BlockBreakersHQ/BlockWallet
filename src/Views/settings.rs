use adw::prelude::*;
use adw::{ApplicationWindow, HeaderBar};
use glib::{clone, ControlFlow};
use gtk::prelude::*;
use gtk::{Button, Orientation};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::configuration::application_settings::*;
use crate::configuration::wallet_store::CustomTokenRecord;
use crate::currencies::eth_chain;
use crate::currencies::ltc_chain;
use crate::currencies::sol_chain;
use crate::views::{login, stack};

// Index 1 ("sepolia") is load-bearing: the "Use test networks" toggle hardcodes
// `eth_network.set_selected(1)` to mean Sepolia.
const ETH_NETWORKS: [&str; 8] = [
    "mainnet", "sepolia", "arbitrum", "base", "optimism", "polygon", "bsc", "avalanche",
];

fn section_label(text: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .halign(gtk::Align::Start)
        .margin_start(16)
        .margin_end(16)
        .margin_top(16)
        .margin_bottom(4)
        .css_classes(["currency-name"])
        .build()
}

pub fn settings_view(window: ApplicationWindow, app_settings: Arc<Mutex<ApplicationSettings>>) {
    let header_bar = HeaderBar::new();
    header_bar.set_title_widget(Some(&gtk::Label::new(Some("Settings"))));
    let back = Button::from_icon_name("go-previous-symbolic");
    back.set_tooltip_text(Some("Back"));
    back.add_css_class("standard_button");
    header_bar.pack_start(&back);

    let page = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .margin_bottom(16)
        .build();

    page.append(&section_label("Security"));
    let timeout_labels = ["Off", "1 minute", "2 minutes", "5 minutes"];
    let timeout_values = [0_u32, 60, 120, 300];
    let timeout = gtk::DropDown::from_strings(&timeout_labels);
    let current_timeout = app_settings.lock().unwrap().lock_timeout_secs;
    let selected = timeout_values
        .iter()
        .position(|v| *v == current_timeout)
        .unwrap_or(2);
    timeout.set_selected(selected as u32);
    timeout.set_margin_start(12);
    timeout.set_margin_end(12);
    page.append(
        &gtk::Label::builder()
            .label("Auto-lock")
            .halign(gtk::Align::Start)
            .margin_start(16)
            .css_classes(["currency-ticker"])
            .build(),
    );
    page.append(&timeout);

    page.append(&section_label("Display"));
    let prices = gtk::Switch::builder()
        .active(app_settings.lock().unwrap().show_prices)
        .halign(gtk::Align::End)
        .margin_start(16)
        .margin_end(16)
        .valign(gtk::Align::Center)
        .build();
    let prices_row = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .margin_start(16)
        .margin_end(16)
        .margin_top(6)
        .build();
    prices_row.append(
        &gtk::Label::builder()
            .label("Show fiat prices (CoinGecko)")
            .wrap(true)
            .hexpand(true)
            .halign(gtk::Align::Start)
            .build(),
    );
    prices_row.append(&prices);
    page.append(&prices_row);

    let fiat = gtk::DropDown::from_strings(&["usd", "eur"]);
    if app_settings.lock().unwrap().fiat.eq_ignore_ascii_case("eur") {
        fiat.set_selected(1);
    }
    fiat.set_margin_start(12);
    fiat.set_margin_end(12);
    page.append(&fiat);

    let units = gtk::DropDown::from_strings(&["btc", "sats"]);
    if app_settings.lock().unwrap().btc_units.eq_ignore_ascii_case("sats") {
        units.set_selected(1);
    }
    units.set_margin_start(12);
    units.set_margin_end(12);
    page.append(&units);

    page.append(&section_label("Network"));
    let network_settings_box = network_settings_box(app_settings.clone());
    network_settings_box.set_visible(true);
    page.append(&network_settings_box);

    let save_display = Button::builder()
        .label("Save display and security")
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .build();
    save_display.add_css_class("standard_button");
    page.append(&save_display);

    let logout_button = Button::builder()
        .label("Lock wallet")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    logout_button.add_css_class("standard_button");
    if app_settings.lock().unwrap().is_unlocked() {
        page.append(&logout_button);
    }

    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(480);
    clamp.set_child(Some(&page));
    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&clamp)
        .build();

    let setting_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();
    setting_box.append(&header_bar);
    setting_box.append(&scroll);
    window.set_content(Some(&setting_box));
    window.present();

    save_display.connect_clicked(clone!(
        #[strong] app_settings,
        #[weak] timeout,
        #[weak] prices,
        #[weak] fiat,
        #[weak] units,
        move |_| {
            let secs = timeout_values
                .get(timeout.selected() as usize)
                .copied()
                .unwrap_or(120);
            let mut settings = app_settings.lock().unwrap();
            settings.lock_timeout_secs = secs;
            settings.show_prices = prices.is_active();
            settings.fiat = if fiat.selected() == 1 { "eur".into() } else { "usd".into() };
            settings.btc_units = if units.selected() == 1 { "sats".into() } else { "btc".into() };
            let _ = settings.write_config();
        }
    ));

    logout_button.connect_clicked(clone!(
        #[weak] window,
        #[strong] app_settings,
        move |_| {
            login::lock_and_show(window.clone(), app_settings.clone());
        }
    ));

    back.connect_clicked(clone!(
        #[weak] window,
        #[strong] app_settings,
        move |_| {
            if app_settings.lock().unwrap().is_unlocked() {
                stack::stack_view(&window, app_settings.lock().unwrap().clone());
            } else {
                login::lock_and_show(window.clone(), app_settings.clone());
            }
        }
    ));
}

pub fn network_settings_box(app_settings: Arc<Mutex<ApplicationSettings>>) -> gtk::Box {
    let network_settings_box = gtk::Box::new(Orientation::Vertical, 0);
    network_settings_box.set_visible(false);

    let testnet_row = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .margin_start(16)
        .margin_end(16)
        .margin_top(8)
        .build();
    testnet_row.append(
        &gtk::Label::builder()
            .label("Use test networks (BTC testnet + ETH Sepolia + SOL devnet + LTC testnet)")
            .wrap(true)
            .hexpand(true)
            .halign(gtk::Align::Start)
            .build(),
    );
    let testnet = gtk::Switch::builder()
        .active(app_settings.lock().unwrap().is_test_mode())
        .valign(gtk::Align::Center)
        .build();
    testnet_row.append(&testnet);

    let etherscan_api_key = gtk::Entry::builder()
        .placeholder_text("Etherscan Api Key")
        .margin_top(12)
        .margin_bottom(3)
        .margin_start(12)
        .margin_end(12)
        .build();

    let infura_key = gtk::Entry::builder()
        .placeholder_text("Infura API Key")
        .margin_top(3)
        .margin_bottom(3)
        .margin_start(12)
        .margin_end(12)
        .build();
    
    let eth_network = gtk::DropDown::from_strings(&ETH_NETWORKS);
    let current_eth = app_settings.lock().unwrap().eth_network.to_ascii_lowercase();
    if let Some(index) = ETH_NETWORKS.iter().position(|n| *n == current_eth) {
        eth_network.set_selected(index as u32);
    }
    eth_network.set_margin_start(12);
    eth_network.set_margin_end(12);
    eth_network.set_margin_top(3);
    eth_network.set_margin_bottom(3);

    let ethereum_node = gtk::Entry::builder()
        .placeholder_text("ETH RPC https://… (empty = default for network)")
        .margin_top(3)
        .margin_bottom(3)
        .margin_start(12)
        .margin_end(12)
        .text(&app_settings.lock().unwrap().eth_node)
        .build();
    
    let eth_incorrect_format = gtk::Label::builder()
        .label("ETH RPC should be http:// or https://, or empty for the network default.")
        .visible(false)
        .wrap(true)
        .css_classes(["label-error"])
        .build();

    let token_contract = gtk::Entry::builder()
        .placeholder_text("Add ERC-20 contract (0x…)")
        .margin_top(3)
        .margin_bottom(3)
        .margin_start(12)
        .margin_end(12)
        .build();
    let add_token = Button::builder()
        .label("Add token")
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(3)
        .build();
    add_token.add_css_class("standard_button");
    let token_status = gtk::Label::builder()
        .wrap(true)
        .visible(false)
        .margin_start(12)
        .margin_end(12)
        .css_classes(["label-standard"])
        .build();

    let sol_network = gtk::DropDown::from_strings(&["mainnet", "devnet"]);
    if app_settings.lock().unwrap().sol_network.to_ascii_lowercase() == "devnet" {
        sol_network.set_selected(1);
    }
    sol_network.set_margin_start(12);
    sol_network.set_margin_end(12);
    sol_network.set_margin_top(3);
    sol_network.set_margin_bottom(3);

    let solana_node = gtk::Entry::builder()
        .placeholder_text("SOL RPC https://… (empty = default for network)")
        .margin_top(3)
        .margin_bottom(3)
        .margin_start(12)
        .margin_end(12)
        .text(&app_settings.lock().unwrap().sol_node)
        .build();

    let sol_incorrect_format = gtk::Label::builder()
        .label("SOL RPC should be http:// or https://, or empty for the network default.")
        .visible(false)
        .wrap(true)
        .css_classes(["label-error"])
        .build();

    let spl_mint = gtk::Entry::builder()
        .placeholder_text("Add SPL token by mint address")
        .margin_top(3)
        .margin_bottom(3)
        .margin_start(12)
        .margin_end(12)
        .build();
    let add_spl_token = Button::builder()
        .label("Add token")
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(3)
        .build();
    add_spl_token.add_css_class("standard_button");
    let spl_status = gtk::Label::builder()
        .wrap(true)
        .visible(false)
        .margin_start(12)
        .margin_end(12)
        .css_classes(["label-standard"])
        .build();

    let btc_network = gtk::DropDown::from_strings(&["bitcoin", "testnet"]);
    if app_settings.lock().unwrap().btc_network.to_ascii_lowercase() == "testnet" {
        btc_network.set_selected(1);
    }
    btc_network.set_margin_start(12);
    btc_network.set_margin_end(12);
    btc_network.set_margin_top(3);
    btc_network.set_margin_bottom(3);

    let btc_node = gtk::Entry::builder()
        .placeholder_text("Esplora https://… or Electrum ssl://host:port (empty = default)")
        .margin_top(3)
        .margin_bottom(3)
        .margin_start(12)
        .margin_end(12)
        .text(&app_settings.lock().unwrap().btc_node)
        .build();

    let btc_incorrect_format = gtk::Label::builder()
        .label("BTC node should be an https Esplora URL or ssl://host:port Electrum server.")
        .visible(false)
        .wrap(true)
        .css_classes(["label-error"])
        .build();

    let ltc_network = gtk::DropDown::from_strings(&["litecoin", "testnet"]);
    if app_settings.lock().unwrap().ltc_network.to_ascii_lowercase() == "testnet" {
        ltc_network.set_selected(1);
    }
    ltc_network.set_margin_start(12);
    ltc_network.set_margin_end(12);
    ltc_network.set_margin_top(3);
    ltc_network.set_margin_bottom(3);

    let litecoin_node = gtk::Entry::builder()
        .placeholder_text("LTC Esplora https://… (empty = default for network)")
        .margin_top(3)
        .margin_bottom(3)
        .margin_start(12)
        .margin_end(12)
        .text(&app_settings.lock().unwrap().ltc_node)
        .build();

    let ltc_incorrect_format = gtk::Label::builder()
        .label("LTC node should be an https Esplora URL, or empty for the network default.")
        .visible(false)
        .wrap(true)
        .css_classes(["label-error"])
        .build();

    let save_button = Button::builder()
        .label("Save Settings")
        .margin_top(3)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    
    network_settings_box.append(&testnet_row);
    network_settings_box.append(&etherscan_api_key);
    network_settings_box.append(&infura_key);
    network_settings_box.append(&eth_network);
    network_settings_box.append(&ethereum_node);
    network_settings_box.append(&eth_incorrect_format);
    network_settings_box.append(&token_contract);
    network_settings_box.append(&add_token);
    network_settings_box.append(&token_status);
    network_settings_box.append(&btc_network);
    network_settings_box.append(&btc_node);
    network_settings_box.append(&btc_incorrect_format);
    network_settings_box.append(&sol_network);
    network_settings_box.append(&solana_node);
    network_settings_box.append(&sol_incorrect_format);
    network_settings_box.append(&spl_mint);
    network_settings_box.append(&add_spl_token);
    network_settings_box.append(&spl_status);
    network_settings_box.append(&ltc_network);
    network_settings_box.append(&litecoin_node);
    network_settings_box.append(&ltc_incorrect_format);
    network_settings_box.append(&save_button);

    add_token.connect_clicked(clone!(
        #[strong] app_settings,
        #[weak] token_contract,
        #[weak] token_status,
        move |_| {
            let contract = token_contract.text().to_string();
            if eth_chain::validate_address(&contract).is_err() {
                token_status.set_label("Token contract must be a 0x address.");
                token_status.set_visible(true);
                return;
            }
            token_status.set_label("Looking up token…");
            token_status.set_visible(true);
            let settings = app_settings.lock().unwrap();
            let node = settings.eth_node.clone();
            let network = settings.eth_network.clone();
            let infura = settings.infura_key.clone();
            drop(settings);
            let (sender, receiver) = crate::configuration::ui_channel::unbounded();
            thread::spawn(move || {
                let result = eth_chain::fetch_token_metadata(&contract, &node, &network, &infura);
                let _ = sender.send_blocking(result);
            });
            crate::configuration::ui_channel::attach(
                receiver,
                clone!(
                    #[strong] app_settings,
                    #[weak] token_status,
                    #[weak] token_contract,
                    #[upgrade_or]
                    ControlFlow::Break,
                    move |result| {
                        match result {
                            Ok(token) => {
                                app_settings.lock().unwrap().add_custom_token(CustomTokenRecord {
                                    symbol: token.symbol.clone(),
                                    name: token.name.clone(),
                                    address: token.address.clone(),
                                    decimals: token.decimals as i32,
                                    chain: "eth".to_string(),
                                });
                                let _ = app_settings.lock().unwrap().write_config();
                                token_status.set_label(&format!("Added {} ({})", token.symbol, token.address));
                                token_contract.set_text("");
                            }
                            Err(_) => {
                                token_status.set_label("Could not read that contract. Check the address and Ethereum RPC.");
                            }
                        }
                        token_status.set_visible(true);
                        ControlFlow::Break
                    }
                ),
            );
        }
    ));

    add_spl_token.connect_clicked(clone!(
        #[strong] app_settings,
        #[weak] spl_mint,
        #[weak] spl_status,
        move |_| {
            let mint = spl_mint.text().to_string();
            if sol_chain::validate_address(&mint).is_err() {
                spl_status.set_label("Mint must be a base58 Solana address.");
                spl_status.set_visible(true);
                return;
            }
            spl_status.set_label("Looking up token…");
            spl_status.set_visible(true);
            let settings = app_settings.lock().unwrap();
            let node = settings.sol_node.clone();
            let network = settings.sol_network.clone();
            drop(settings);
            let (sender, receiver) = crate::configuration::ui_channel::unbounded();
            thread::spawn(move || {
                let result = sol_chain::fetch_token_metadata(&mint, &node, &network);
                let _ = sender.send_blocking(result);
            });
            crate::configuration::ui_channel::attach(
                receiver,
                clone!(
                    #[strong] app_settings,
                    #[weak] spl_status,
                    #[weak] spl_mint,
                    #[upgrade_or]
                    ControlFlow::Break,
                    move |result| {
                        match result {
                            Ok(token) => {
                                app_settings.lock().unwrap().add_custom_token(CustomTokenRecord {
                                    symbol: token.symbol.clone(),
                                    name: token.name.clone(),
                                    address: token.address.clone(),
                                    decimals: token.decimals as i32,
                                    chain: "sol".to_string(),
                                });
                                let _ = app_settings.lock().unwrap().write_config();
                                spl_status.set_label(&format!("Added {} ({})", token.symbol, token.address));
                                spl_mint.set_text("");
                            }
                            Err(_) => {
                                spl_status.set_label("Could not read that mint. Check the address and Solana RPC.");
                            }
                        }
                        spl_status.set_visible(true);
                        ControlFlow::Break
                    }
                ),
            );
        }
    ));

    save_button.connect_clicked(move |_| {
        eth_incorrect_format.set_visible(false);
        btc_incorrect_format.set_visible(false);
        sol_incorrect_format.set_visible(false);
        ltc_incorrect_format.set_visible(false);
        if !etherscan_api_key.text().is_empty() {
            app_settings.lock().unwrap().etherscan_key = etherscan_api_key.text().to_string();
        }
        let eth_text = ethereum_node.text().to_string();
        if eth_text.is_empty()
            || eth_text.starts_with("http://")
            || eth_text.starts_with("https://")
        {
            app_settings.lock().unwrap().eth_node = eth_text;
        } else {
            eth_incorrect_format.set_visible(true);
        }
        if testnet.is_active() {
            app_settings.lock().unwrap().apply_test_networks(true);
            btc_network.set_selected(1);
            eth_network.set_selected(1);
            sol_network.set_selected(1);
            ltc_network.set_selected(1);
        } else {
            let selected_eth = ETH_NETWORKS
                .get(eth_network.selected() as usize)
                .unwrap_or(&"mainnet");
            app_settings.lock().unwrap().apply_eth_network(selected_eth);
            let networks = ["bitcoin", "testnet"];
            let selected_network = networks
                .get(btc_network.selected() as usize)
                .unwrap_or(&"bitcoin");
            let _ = app_settings.lock().unwrap().apply_btc_network(selected_network);
            let sol_networks = ["mainnet", "devnet"];
            let selected_sol = sol_networks
                .get(sol_network.selected() as usize)
                .unwrap_or(&"mainnet");
            app_settings.lock().unwrap().apply_sol_network(selected_sol);
            let ltc_networks = ["litecoin", "testnet"];
            let selected_ltc = ltc_networks
                .get(ltc_network.selected() as usize)
                .unwrap_or(&"litecoin");
            app_settings.lock().unwrap().apply_ltc_network(selected_ltc);
        }
        let btc_text = btc_node.text().to_string();
        if btc_text.is_empty()
            || btc_text.starts_with("http://")
            || btc_text.starts_with("https://")
            || btc_text.starts_with("ssl://")
            || btc_text.starts_with("tcp://")
        {
            app_settings.lock().unwrap().btc_node = btc_text;
        } else {
            btc_incorrect_format.set_visible(true);
        }
        let sol_text = solana_node.text().to_string();
        if sol_text.is_empty()
            || sol_text.starts_with("http://")
            || sol_text.starts_with("https://")
        {
            app_settings.lock().unwrap().sol_node = sol_text;
        } else {
            sol_incorrect_format.set_visible(true);
        }
        let ltc_text = litecoin_node.text().to_string();
        if ltc_text.is_empty()
            || ltc_text.starts_with("http://")
            || ltc_text.starts_with("https://")
        {
            app_settings.lock().unwrap().ltc_node = ltc_text;
        } else {
            ltc_incorrect_format.set_visible(true);
        }
        if !infura_key.text().is_empty() {
            app_settings.lock().unwrap().infura_key = infura_key.text().to_string();
        }
        let _ = app_settings.lock().unwrap().write_config();
    });
    
    return network_settings_box;
}