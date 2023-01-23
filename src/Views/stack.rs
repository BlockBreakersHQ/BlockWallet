use gtk::prelude::*;
use adw::prelude::*;
use adw::{ApplicationWindow};
use std::thread;
use std::time::Duration;

use crate::views::{wallets, header_bar};
use crate::configuration::application_settings::*;
use crate::currencies::eth::EthereumWallet;
use crate::currencies::btc::BitcoinWallet;

pub fn stack_view(window: &ApplicationWindow, app_settings: ApplicationSettings) {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    window.set_content(Some(&container));

    let header_bar = header_bar::header_bar_view(window.clone(), app_settings.clone());
    let stack = adw::ViewStack::new();
    let mut app_settings_clone = app_settings.clone();

    let home_label = gtk::Label::new(Some("Home"));
    stack.add_titled(&home_label, Option::<&str>::None, "Home");

    let opt: Option<&str> = Some("Wallets");
    stack.add_titled(&wallets::wallet_view(app_settings), opt, "Wallets");

    let asset_label = gtk::Label::new(Some("Assets"));
    stack.add_titled(&asset_label, Option::<&str>::None, "Assets");

    let trade_label = gtk::Label::new(Some("Trade"));
    stack.add_titled(&trade_label, Option::<&str>::None, "Trade");

    let stack_bar = adw::ViewSwitcherBar::new();
    stack_bar.set_stack(Some(&stack));
    stack_bar.set_reveal(true);

    container.append(&header_bar);
    container.append(&stack_bar);
    container.append(&stack);

    println!("Inside Stack!");
    let _ = app_settings_clone.write_config();
    window.show();
    let app_settings_update = app_settings_clone.clone();

    thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let btc_wallet_count = app_settings_update.btc_wallets.len();
        let eth_wallet_count = app_settings_update.eth_wallets.len();

        loop {
            std::thread::sleep(Duration::from_millis(1000));
            let mut btc_wallets = app_settings_update.btc_wallets.clone();
            let mut eth_wallets = app_settings_update.eth_wallets.clone();
            let _ = runtime.block_on(runtime.spawn(async move {
                for i in 0..btc_wallet_count {
                    let btc_address_raw = match &btc_wallets[i].address {
                        Some(address) => String::from(address),
                        None => continue
                    };
    
                    btc_wallets[i].balance = match BitcoinWallet::get_balance(btc_address_raw).await {
                        Some(sat) => Some((sat.parse::<f64>().unwrap() / 100000000.0).to_string()),
                        None => Some(String::from("0"))
                    };
                }
                
                for i in 0..eth_wallet_count {
                    let eth_address_raw = match &eth_wallets[i].address {
                        Some(address) => String::from(address),
                        None => continue
                    };
    
                    eth_wallets[i].balance = match EthereumWallet::get_balance(eth_address_raw).await {
                        Some(gwei) => Some((gwei.parse::<f64>().unwrap() / 1000000000000000000.0).to_string()),
                        None => Some(String::from("0"))
                    };
                }
            }));
        }
    });
}