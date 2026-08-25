use gtk::prelude::*;
use gtk::{Orientation, Button, Align};
use pango::WrapMode;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;
use glib::{clone, ControlFlow};

use crate::currencies::btc::BitcoinWallet;
use crate::currencies::btc_chain;
use crate::currencies::tokens::Token;
use crate::currencies::eth::EthereumWallet;
use crate::currencies::eth_chain;
use crate::currencies::ltc::LitecoinWallet;
use crate::currencies::ltc_chain;
use crate::currencies::sol::SolanaWallet;
use crate::ApplicationSettings;
use crate::views::home::generate_currency_box_static;
use crate::views::nav::{self, Nav};
use crate::views::transactions;

pub fn currency_view(token: Token, app_settings: ApplicationSettings, nav: Option<Nav>) -> gtk::Box {
    let currency_box = gtk::Box::new(Orientation::Vertical, 0);
    let display = live_balance_label(&token, &app_settings);
    let currency_label = generate_currency_box_static(&token, &display);
    let offline = nav::label_is_offline(&display);
    let banner = nav::banner(if offline {
        "Node unreachable. You can still receive at the address below."
    } else {
        "Send needs a reachable node. Receive works offline."
    });

    let transactions_box = get_transactions(token.clone(), app_settings.clone());
    let receive_box = match token.chain.as_str() {
        "btc" => generate_btc_receive_box(&app_settings.btc_wallets),
        "sol" => generate_sol_receive_box(&app_settings.sol_wallets),
        "ltc" => generate_ltc_receive_box(&app_settings.ltc_wallets),
        _ => generate_eth_receive_box(&app_settings.eth_wallets),
    };

    let send_button = Button::builder()
        .label(&format!("Send {}", &token.symbol))
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .build();
    send_button.add_css_class("standard_button");

    let receive_button = Button::builder()
        .label(&format!("Receive {}", &token.symbol))
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .build();
    receive_button.add_css_class("standard_button");

    let currency_detail_box = gtk::Box::new(Orientation::Vertical, 0);
    currency_detail_box.append(&currency_label);
    currency_detail_box.append(&banner);
    currency_detail_box.append(&send_button);
    currency_detail_box.append(&receive_button);
    currency_detail_box.append(&receive_box);
    currency_detail_box.append(&transactions_box);

    let scrollable_container = gtk::ScrolledWindow::builder()
        .child(&currency_detail_box)
        .vexpand(true)
        .build();
    currency_box.append(&scrollable_container);

    send_button.connect_clicked(clone!(
        #[strong] token,
        #[strong] app_settings,
        #[strong] nav,
        move |_| {
            let send = transactions::transaction_view(app_settings.clone(), token.clone()).0;
            if let Some(nav) = &nav {
                nav.push("send", &send);
            } else {
                // Fallback when opened without a tab stack.
            }
        }
    ));

    receive_button.connect_clicked(move |_| {
        receive_box.set_visible(!receive_box.is_visible());
    });

    let balance_arc = balance_watch_arc(&token, &app_settings);
    let (sender, receiver) = crate::configuration::ui_channel::unbounded();
    thread::spawn(move || {
        loop {
            let text = balance_arc.lock().unwrap().clone();
            if sender.send_blocking(text).is_err() {
                break;
            }
            thread::sleep(Duration::from_secs(2));
        }
    });
    crate::configuration::ui_channel::attach(
        receiver,
        clone!(
            #[weak] banner,
            #[upgrade_or]
            ControlFlow::Break,
            move |text| {
                if nav::label_is_offline(&text) {
                    banner.set_label("Node unreachable. You can still receive at the address below.");
                }
                ControlFlow::Continue
            }
        ),
    );

    currency_box
}

fn live_balance_label(token: &Token, app_settings: &ApplicationSettings) -> String {
    match token.chain.as_str() {
        "btc" => {
            return app_settings
                .btc_wallets
                .first()
                .map(|w| nav::format_btc_units(&w.balance.lock().unwrap(), &app_settings.btc_units))
                .unwrap_or_else(|| "0 BTC".into());
        }
        "sol" => {
            if token.symbol == "SOL" {
                return app_settings
                    .sol_wallets
                    .first()
                    .map(|w| w.balance.lock().unwrap().clone())
                    .unwrap_or_else(|| "0 SOL".into());
            }
            return app_settings
                .sol_wallets
                .iter()
                .find_map(|w| w.spl_balances.lock().unwrap().get(&token.symbol).copied())
                .map(|amount| format!("{amount} {}", token.symbol))
                .unwrap_or_else(|| format!("0 {}", token.symbol));
        }
        "ltc" => {
            return app_settings
                .ltc_wallets
                .first()
                .map(|w| w.balance.lock().unwrap().clone())
                .unwrap_or_else(|| "0 LTC".into());
        }
        _ => {}
    }
    if token.chain == "eth" && eth_chain::is_native_token(token) {
        return app_settings
            .eth_wallets
            .first()
            .map(|w| w.balance.lock().unwrap().clone())
            .unwrap_or_else(|| format!("0 {}", token.symbol));
    }
    app_settings
        .eth_wallets
        .iter()
        .find_map(|w| {
            w.erc20_balances
                .lock()
                .unwrap()
                .get(&token.symbol)
                .copied()
        })
        .map(|amount| format!("{amount} {}", token.symbol))
        .unwrap_or_else(|| format!("0 {}", token.symbol))
}

fn balance_watch_arc(token: &Token, app_settings: &ApplicationSettings) -> Arc<Mutex<String>> {
    if token.chain == "btc" {
        if let Some(w) = app_settings.btc_wallets.first() {
            return Arc::clone(&w.balance);
        }
    } else if token.chain == "sol" && token.symbol == "SOL" {
        if let Some(w) = app_settings.sol_wallets.first() {
            return Arc::clone(&w.balance);
        }
    } else if token.chain == "ltc" {
        if let Some(w) = app_settings.ltc_wallets.first() {
            return Arc::clone(&w.balance);
        }
    } else if token.chain == "eth" && eth_chain::is_native_token(token) {
        if let Some(w) = app_settings.eth_wallets.first() {
            return Arc::clone(&w.balance);
        }
    }
    Arc::new(Mutex::new(live_balance_label(token, app_settings)))
}

/*
pub fn get_balances_by_wallet(token: Token, app_settings: ApplicationSettings) -> gtk::Box {
    let wallet_balances_box = gtk::Box::new(Orientation::Vertical, 0);
    return wallet_balances_box;
}
*/

pub fn get_transactions(token: Token, app_settings: ApplicationSettings) -> gtk::Box {
    let transactions_box = gtk::Box::new(Orientation::Vertical, 0);
    let mut transaction_counter = 0;
    
    let no_transactions_label = gtk::Label::builder()
        .label("No transactions to display.")
        .margin_top(5)
        .margin_start(5)
        .css_classes(["currency-name"])
        .visible(false)
        .name("no_transactions_label")
        .build();

    transactions_box.append(&no_transactions_label);

    if token.symbol == "BTC" {
        for btcw in &app_settings.btc_wallets {
            let history = btcw.history.lock().unwrap().clone();
            for item in history {
                transaction_counter += 1;
                let row = gtk::Box::new(Orientation::Horizontal, 0);
                row.set_margin_bottom(5);
                row.set_margin_start(5);
                row.set_margin_end(5);
                let label = if item.amount_sats >= 0 {
                    format!(
                        "+{} BTC  {} conf  {}",
                        btc_chain::format_btc(item.amount_sats as u64),
                        item.confirmations,
                        &item.txid[..item.txid.len().min(12)]
                    )
                } else {
                    format!(
                        "-{} BTC  {} conf  {}",
                        btc_chain::format_btc(item.amount_sats.unsigned_abs()),
                        item.confirmations,
                        &item.txid[..item.txid.len().min(12)]
                    )
                };
                let css = if item.amount_sats >= 0 {
                    "transaction-positive"
                } else {
                    "transaction-negative"
                };
                let amount_label = gtk::Label::builder()
                    .label(&label)
                    .wrap(true)
                    .halign(Align::Start)
                    .css_classes([css])
                    .build();
                row.append(&amount_label);
                transactions_box.append(&row);
            }
        }
        if transaction_counter == 0 {
            no_transactions_label.set_visible(true);
        }
        return transactions_box;
    }

    if token.symbol == "LTC" {
        for ltcw in &app_settings.ltc_wallets {
            let history = ltcw.history.lock().unwrap().clone();
            for item in history {
                transaction_counter += 1;
                let row = gtk::Box::new(Orientation::Horizontal, 0);
                row.set_margin_bottom(5);
                row.set_margin_start(5);
                row.set_margin_end(5);
                let label = if item.amount_sats >= 0 {
                    format!(
                        "+{} LTC  {} conf  {}",
                        ltc_chain::format_ltc(item.amount_sats as u64),
                        item.confirmations,
                        &item.txid[..item.txid.len().min(12)]
                    )
                } else {
                    format!(
                        "-{} LTC  {} conf  {}",
                        ltc_chain::format_ltc(item.amount_sats.unsigned_abs()),
                        item.confirmations,
                        &item.txid[..item.txid.len().min(12)]
                    )
                };
                let css = if item.amount_sats >= 0 {
                    "transaction-positive"
                } else {
                    "transaction-negative"
                };
                let amount_label = gtk::Label::builder()
                    .label(&label)
                    .wrap(true)
                    .halign(Align::Start)
                    .css_classes([css])
                    .build();
                row.append(&amount_label);
                transactions_box.append(&row);
            }
        }
        if transaction_counter == 0 {
            no_transactions_label.set_visible(true);
        }
        return transactions_box;
    }

    let wallet_history: Vec<_> = if token.chain == "sol" {
        app_settings
            .sol_wallets
            .iter()
            .flat_map(|w| w.history.lock().unwrap().clone())
            .map(|item| (item.symbol, item.amount, item.incoming, item.confirmations, item.txid))
            .collect()
    } else {
        app_settings
            .eth_wallets
            .iter()
            .flat_map(|w| w.history.lock().unwrap().clone())
            .map(|item| (item.symbol, item.amount, item.incoming, item.confirmations, item.txid))
            .collect()
    };

    for (symbol, amount, incoming, confirmations, txid) in wallet_history {
        if symbol != token.symbol {
            continue;
        }
        transaction_counter += 1;
        let row = gtk::Box::new(Orientation::Horizontal, 0);
        row.set_margin_bottom(5);
        row.set_margin_start(5);
        row.set_margin_end(5);
        let sign = if incoming { "+" } else { "-" };
        let css = if incoming {
            "transaction-positive"
        } else {
            "transaction-negative"
        };
        let txid = if txid.len() > 12 { &txid[..12] } else { &txid };
        let amount_label = gtk::Label::builder()
            .label(&format!("{sign}{} {}  {} conf  {}", amount, symbol, confirmations, txid))
            .wrap(true)
            .halign(Align::Start)
            .css_classes([css])
            .build();
        row.append(&amount_label);
        transactions_box.append(&row);
    }

    if transaction_counter == 0 {
        no_transactions_label.set_visible(true);
    }
    return transactions_box;
}

pub fn generate_btc_receive_box(btc_wallets: &[BitcoinWallet]) -> gtk::Box {
    let receive_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .visible(false)
        .margin_top(5)
        .margin_bottom(5)
        .margin_start(12)
        .margin_end(12)
        .css_classes(["receiver-box"])
        .build();
    receive_box.append(
        &gtk::Label::builder()
            .label("Receive Bitcoin at this address (works offline):")
            .wrap(true)
            .build(),
    );
    for wallet in btc_wallets {
        let address = wallet.address.clone().unwrap_or_default();
        receive_box.append(
            &gtk::Label::builder()
                .label(&address)
                .selectable(true)
                .wrap(true)
                .wrap_mode(WrapMode::Char)
                .build(),
        );
        if let Ok(qr) = wallet.generate_qr_address() {
            let image = gtk::Image::from_paintable(Some(&qr));
            image.set_pixel_size(160);
            receive_box.append(&image);
        }
    }
    receive_box
}

pub fn generate_eth_receive_box(eth_wallets: &Vec<EthereumWallet>) -> gtk::Box {
    let receive_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .visible(false)
        .margin_top(5)
        .margin_bottom(5)
        .margin_start(12)
        .margin_end(12)
        .css_classes(["receiver-box"])
        .name("reciever_box")
        .build();

    receive_box.append(
        &gtk::Label::builder()
            .label("Receive Ethereum at this address (works offline):")
            .wrap(true)
            .build(),
    );

    for ethw in eth_wallets {
        let address = ethw.address.clone().unwrap_or_default();
        receive_box.append(
            &gtk::Label::builder()
                .label(&address)
                .selectable(true)
                .wrap(true)
                .wrap_mode(WrapMode::Char)
                .build(),
        );
        if let Ok(qr) = ethw.generate_qr_address() {
            let image = gtk::Image::from_paintable(Some(&qr));
            image.set_pixel_size(160);
            receive_box.append(&image);
        }
    }

    receive_box
}

pub fn generate_sol_receive_box(sol_wallets: &Vec<SolanaWallet>) -> gtk::Box {
    let receive_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .visible(false)
        .margin_top(5)
        .margin_bottom(5)
        .margin_start(12)
        .margin_end(12)
        .css_classes(["receiver-box"])
        .name("reciever_box")
        .build();

    receive_box.append(
        &gtk::Label::builder()
            .label("Receive Solana at this address (works offline):")
            .wrap(true)
            .build(),
    );

    for solw in sol_wallets {
        let address = solw.address.clone().unwrap_or_default();
        receive_box.append(
            &gtk::Label::builder()
                .label(&address)
                .selectable(true)
                .wrap(true)
                .wrap_mode(WrapMode::Char)
                .build(),
        );
        if let Ok(qr) = solw.generate_qr_address() {
            let image = gtk::Image::from_paintable(Some(&qr));
            image.set_pixel_size(160);
            receive_box.append(&image);
        }
    }

    receive_box
}

pub fn generate_ltc_receive_box(ltc_wallets: &Vec<LitecoinWallet>) -> gtk::Box {
    let receive_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .visible(false)
        .margin_top(5)
        .margin_bottom(5)
        .margin_start(12)
        .margin_end(12)
        .css_classes(["receiver-box"])
        .build();
    receive_box.append(
        &gtk::Label::builder()
            .label("Receive Litecoin at this address (works offline):")
            .wrap(true)
            .build(),
    );
    for wallet in ltc_wallets {
        let address = wallet.address.clone().unwrap_or_default();
        receive_box.append(
            &gtk::Label::builder()
                .label(&address)
                .selectable(true)
                .wrap(true)
                .wrap_mode(WrapMode::Char)
                .build(),
        );
        if let Ok(qr) = wallet.generate_qr_address() {
            let image = gtk::Image::from_paintable(Some(&qr));
            image.set_pixel_size(160);
            receive_box.append(&image);
        }
    }
    receive_box
}
