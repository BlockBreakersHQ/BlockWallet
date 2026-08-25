use glib::{clone, ControlFlow};
use gtk::prelude::*;
use gtk::Orientation;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::currencies::eth_chain;
use crate::currencies::tokens::Token;
use crate::views::currency::currency_view;
use crate::views::home::generate_currency_box_static;
use crate::views::nav::{self, Nav};
use crate::ApplicationSettings;

pub fn asset_view(
    app_settings: Arc<Mutex<ApplicationSettings>>,
) -> (gtk::Box, Nav, Arc<Mutex<ApplicationSettings>>) {
    let list = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .build();
    let banner = nav::banner("Tap an asset to send or receive. Zero balances stay listed so you can receive.");
    let empty = gtk::Label::builder()
        .label("Unlock a wallet to see assets. Receive still works offline.")
        .wrap(true)
        .css_classes(["currency-name"])
        .margin_top(16)
        .visible(false)
        .build();
    let rows = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();
    list.append(&banner);
    list.append(&empty);
    list.append(&rows);

    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&list)
        .build();
    let nav = Nav::new(&scroll);
    let page = nav.clone().wrap();

    let (sender, receiver) = crate::configuration::ui_channel::unbounded();
    let app = app_settings.clone();
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(4));
            let snapshot = app.lock().unwrap().clone();
            let units = snapshot.btc_units.clone();
            let mut items: Vec<(Token, String)> = Vec::new();
            let mut offline = false;

            if let Some(token) = snapshot.tokens.eth_tokens.get("btc:BTC").cloned() {
                let mut display = snapshot
                    .btc_wallets
                    .iter()
                    .map(|w| nav::parse_leading_amount(&w.balance.lock().unwrap()))
                    .sum::<f64>()
                    .to_string();
                if snapshot.btc_wallets.iter().any(|w| nav::label_is_offline(&w.balance.lock().unwrap())) {
                    offline = true;
                    display = format!("{display} BTC (offline)");
                } else if snapshot
                    .btc_wallets
                    .iter()
                    .all(|w| nav::label_is_pending_sync(&w.balance.lock().unwrap()))
                    && !snapshot.btc_wallets.is_empty()
                {
                    display = "Syncing…".into();
                } else {
                    display = nav::format_btc_units(&format!("{display} BTC"), &units);
                }
                items.push((token, display));
            }
            let eth_native = snapshot
                .tokens
                .eth_tokens
                .values()
                .find(|t| t.chain == "eth" && eth_chain::is_native_token(t))
                .cloned();
            if let Some(token) = eth_native {
                let symbol = token.symbol.clone();
                let mut display = snapshot
                    .eth_wallets
                    .iter()
                    .map(|w| nav::parse_leading_amount(&w.balance.lock().unwrap()))
                    .sum::<f64>()
                    .to_string();
                if snapshot.eth_wallets.iter().any(|w| nav::label_is_offline(&w.balance.lock().unwrap())) {
                    offline = true;
                    display = format!("{display} {symbol} (offline)");
                } else if snapshot
                    .eth_wallets
                    .iter()
                    .all(|w| nav::label_is_pending_sync(&w.balance.lock().unwrap()))
                    && !snapshot.eth_wallets.is_empty()
                {
                    display = "Syncing…".into();
                } else {
                    display = format!("{display} {symbol}");
                }
                items.push((token, display));
            }
            for wallet in &snapshot.eth_wallets {
                let erc20 = wallet.erc20_balances.lock().unwrap().clone();
                for (symbol, amount) in erc20 {
                    if amount <= 0.0 {
                        continue;
                    }
                    if let Some(token) = snapshot.tokens.eth_tokens.get(&format!("eth:{symbol}")).cloned() {
                        items.push((token, format!("{amount} {symbol}")));
                    }
                }
            }
            if let Some(token) = snapshot.tokens.eth_tokens.get("sol:SOL").cloned() {
                let mut display = snapshot
                    .sol_wallets
                    .iter()
                    .map(|w| nav::parse_leading_amount(&w.balance.lock().unwrap()))
                    .sum::<f64>()
                    .to_string();
                if snapshot.sol_wallets.iter().any(|w| nav::label_is_offline(&w.balance.lock().unwrap())) {
                    offline = true;
                    display = format!("{display} SOL (offline)");
                } else if snapshot
                    .sol_wallets
                    .iter()
                    .all(|w| nav::label_is_pending_sync(&w.balance.lock().unwrap()))
                    && !snapshot.sol_wallets.is_empty()
                {
                    display = "Syncing…".into();
                } else {
                    display = format!("{display} SOL");
                }
                items.push((token, display));
            }
            for wallet in &snapshot.sol_wallets {
                let spl = wallet.spl_balances.lock().unwrap().clone();
                for (symbol, amount) in spl {
                    if amount <= 0.0 {
                        continue;
                    }
                    if let Some(token) = snapshot.tokens.eth_tokens.get(&format!("sol:{symbol}")).cloned() {
                        items.push((token, format!("{amount} {symbol}")));
                    }
                }
            }
            if let Some(token) = snapshot.tokens.eth_tokens.get("ltc:LTC").cloned() {
                let mut display = snapshot
                    .ltc_wallets
                    .iter()
                    .map(|w| nav::parse_leading_amount(&w.balance.lock().unwrap()))
                    .sum::<f64>()
                    .to_string();
                if snapshot.ltc_wallets.iter().any(|w| nav::label_is_offline(&w.balance.lock().unwrap())) {
                    offline = true;
                    display = format!("{display} LTC (offline)");
                } else if snapshot
                    .ltc_wallets
                    .iter()
                    .all(|w| nav::label_is_pending_sync(&w.balance.lock().unwrap()))
                    && !snapshot.ltc_wallets.is_empty()
                {
                    display = "Syncing…".into();
                } else {
                    display = format!("{display} LTC");
                }
                items.push((token, display));
            }
            if sender.send_blocking((items, offline, snapshot)).is_err() {
                break;
            }
        }
    });

    crate::configuration::ui_channel::attach(
        receiver,
        clone!(
            #[weak] rows,
            #[weak] empty,
            #[weak] banner,
            #[strong] nav,
            #[upgrade_or]
            ControlFlow::Break,
            move |(items, offline, snapshot): (Vec<(Token, String)>, bool, ApplicationSettings)| {
                while let Some(child) = rows.first_child() {
                    rows.remove(&child);
                }
                empty.set_visible(items.is_empty());
                banner.set_label(if offline {
                    "A node is unreachable. You can still open an asset to receive."
                } else {
                    "Tap an asset to send or receive. Zero balances stay listed so you can receive."
                });
                for (token, display) in items {
                    let row = generate_currency_box_static(&token, &display);
                    let gesture = gtk::GestureClick::new();
                    let nav = nav.clone();
                    let token_c = token.clone();
                    let settings = snapshot.clone();
                    gesture.connect_released(move |gesture, _, _, _| {
                        gesture.set_state(gtk::EventSequenceState::Claimed);
                        nav.push(
                            "detail",
                            &currency_view(token_c.clone(), settings.clone(), Some(nav.clone())),
                        );
                    });
                    row.add_controller(gesture);
                    rows.append(&row);
                }
                ControlFlow::Continue
            }
        ),
    );

    (page, nav, app_settings)
}

pub fn generate_currency_box(balance: f64, token: Token) -> gtk::Box {
    generate_currency_box_static(&token, &balance.to_string())
}
