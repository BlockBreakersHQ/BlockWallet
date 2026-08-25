use adw::prelude::*;
use glib::{clone, ControlFlow};
use gtk::{Align, Image, Orientation};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::currencies::eth_chain;
use crate::currencies::tokens::Token;
use crate::views::currency::currency_view;
use crate::views::nav::{self, Nav};
use crate::ApplicationSettings;

pub fn home_view(app_settings: Arc<Mutex<ApplicationSettings>>) -> (gtk::Box, Nav, Arc<Mutex<ApplicationSettings>>) {
    let list = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .build();
    let banner = nav::banner("Balances come from your Bitcoin and Ethereum nodes.");
    let rows = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();
    list.append(&banner);
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
        let mut price_cache: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        let mut price_ticks = 0u32;
        loop {
            let snapshot = app.lock().unwrap().clone();
            let units = snapshot.btc_units.clone();
            let show_prices = snapshot.show_prices;
            let fiat = snapshot.fiat.clone();
            let mut items: Vec<(Token, String, bool)> = Vec::new();
            let mut offline = false;
            let mut syncing = false;

            if let Some(token) = snapshot.tokens.eth_tokens.get("btc:BTC").cloned() {
                let mut display = snapshot
                    .btc_wallets
                    .first()
                    .map(|w| w.balance.lock().unwrap().clone())
                    .unwrap_or_else(|| "0 BTC".into());
                if nav::label_is_offline(&display) {
                    offline = true;
                }
                if nav::label_is_pending_sync(&display) {
                    syncing = true;
                    display = "Syncing…".into();
                } else {
                    display = nav::format_btc_units(&display, &units);
                }
                items.push((token, display, false));
            }
            let eth_native = snapshot
                .tokens
                .eth_tokens
                .values()
                .find(|t| t.chain == "eth" && eth_chain::is_native_token(t))
                .cloned();
            if let Some(token) = eth_native {
                let fallback = format!("0 {}", token.symbol);
                let mut display = snapshot
                    .eth_wallets
                    .first()
                    .map(|w| w.balance.lock().unwrap().clone())
                    .unwrap_or(fallback);
                if nav::label_is_offline(&display) {
                    offline = true;
                }
                if nav::label_is_pending_sync(&display) {
                    syncing = true;
                    display = "Syncing…".into();
                }
                items.push((token, display, false));
            }
            if let Some(token) = snapshot.tokens.eth_tokens.get("sol:SOL").cloned() {
                let mut display = snapshot
                    .sol_wallets
                    .first()
                    .map(|w| w.balance.lock().unwrap().clone())
                    .unwrap_or_else(|| "0 SOL".into());
                if nav::label_is_offline(&display) {
                    offline = true;
                }
                if nav::label_is_pending_sync(&display) {
                    syncing = true;
                    display = "Syncing…".into();
                }
                items.push((token, display, false));
            }
            if let Some(token) = snapshot.tokens.eth_tokens.get("ltc:LTC").cloned() {
                let mut display = snapshot
                    .ltc_wallets
                    .first()
                    .map(|w| w.balance.lock().unwrap().clone())
                    .unwrap_or_else(|| "0 LTC".into());
                if nav::label_is_offline(&display) {
                    offline = true;
                }
                if nav::label_is_pending_sync(&display) {
                    syncing = true;
                    display = "Syncing…".into();
                }
                items.push((token, display, false));
            }

            if show_prices && !fiat.is_empty() {
                if price_ticks == 0 {
                    if let Ok(prices) = crate::currencies::prices::fetch_prices(&["BTC", "ETH", "SOL", "LTC"], &fiat) {
                        price_cache = prices;
                    }
                }
                price_ticks = (price_ticks + 1) % 15;
                for (token, display, _) in items.iter_mut() {
                    if let Some(price) = price_cache.get(&token.symbol) {
                        let qty = nav::parse_leading_amount(display);
                        if qty > 0.0 {
                            *display = format!(
                                "{display}  ·  {}",
                                crate::currencies::prices::format_fiat(qty * price, &fiat)
                            );
                        }
                    }
                }
            }

            if sender.send_blocking((items, offline, syncing, snapshot)).is_err() {
                break;
            }
            thread::sleep(Duration::from_secs(4));
        }
    });

    crate::configuration::ui_channel::attach(
        receiver,
        clone!(
            #[weak] rows,
            #[weak] banner,
            #[strong] nav,
            #[upgrade_or]
            ControlFlow::Break,
            move |(items, offline, syncing, snapshot): (Vec<(Token, String, bool)>, bool, bool, ApplicationSettings)| {
                while let Some(child) = rows.first_child() {
                    rows.remove(&child);
                }
                banner.set_label(if offline {
                    "A node is unreachable. Receive still works offline."
                } else if syncing {
                    "Syncing balances from your nodes…"
                } else {
                    "Balances come from your Bitcoin and Ethereum nodes."
                });
                for (token, display, _) in items {
                    let row = generate_currency_box_static(&token, &display);
                    let gesture = gtk::GestureClick::new();
                    let nav = nav.clone();
                    let token_c = token.clone();
                    let settings = snapshot.clone();
                    gesture.connect_released(move |gesture, _, _, _| {
                        gesture.set_state(gtk::EventSequenceState::Claimed);
                        nav.push("detail", &currency_view(token_c.clone(), settings.clone(), Some(nav.clone())));
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

pub fn generate_currency_box(element: (Token, Arc<Mutex<String>>)) -> gtk::Box {
    let row = generate_currency_box_static(&element.0, &element.1.lock().unwrap());
    let (sender, receiver) = crate::configuration::ui_channel::unbounded();
    thread::spawn(move || {
        loop {
            let out_string = element.1.lock().unwrap().clone();
            if sender.send_blocking(out_string).is_err() {
                break;
            }
            thread::sleep(Duration::from_secs(1));
        }
    });
    crate::configuration::ui_channel::attach(
        receiver,
        clone!(
            #[weak] row,
            #[upgrade_or]
            ControlFlow::Break,
            move |price_text| {
                if let Some(label) = row.last_child().and_then(|w| w.first_child()) {
                    if let Ok(label) = label.downcast::<gtk::Label>() {
                        if price_text != "Uninitialized" {
                            label.set_label(&price_text);
                        }
                    }
                }
                ControlFlow::Continue
            }
        ),
    );
    row
}

pub fn generate_currency_box_static(token: &Token, display: &str) -> gtk::Box {
    let currency_box = gtk::Box::new(Orientation::Horizontal, 8);
    currency_box.set_css_classes(&["currency-row"]);
    currency_box.set_margin_start(4);
    currency_box.set_margin_end(4);

    let icon = Image::from_file(&token.logo);
    icon.set_pixel_size(40);
    icon.set_margin_start(8);
    icon.set_margin_top(8);
    icon.set_margin_bottom(8);

    let name_box = gtk::Box::new(Orientation::Vertical, 0);
    name_box.set_hexpand(true);
    name_box.append(
        &gtk::Label::builder()
            .label(&token.name)
            .halign(Align::Start)
            .css_classes(["currency-name"])
            .build(),
    );
    name_box.append(
        &gtk::Label::builder()
            .label(&token.symbol)
            .halign(Align::Start)
            .css_classes(["currency-ticker"])
            .build(),
    );

    let price_box = gtk::Box::new(Orientation::Vertical, 0);
    price_box.append(
        &gtk::Label::builder()
            .label(display)
            .halign(Align::End)
            .wrap(true)
            .max_width_chars(18)
            .css_classes(["currency-price"])
            .build(),
    );

    currency_box.append(&icon);
    currency_box.append(&name_box);
    currency_box.append(&price_box);
    currency_box
}
