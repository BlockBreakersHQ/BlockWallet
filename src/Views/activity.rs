use gtk::prelude::*;
use gtk::{Align, Orientation};
use glib::{clone, ControlFlow};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::ApplicationSettings;
use crate::views::nav;

pub fn activity_view(app_settings: Arc<Mutex<ApplicationSettings>>) -> gtk::Box {
    let page = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(8)
        .margin_bottom(8)
        .build();

    let banner = nav::banner("Activity is recent history from this device. Receive still works if a node is unreachable.");
    let empty = gtk::Label::builder()
        .label("No activity yet.")
        .css_classes(["currency-name"])
        .margin_top(16)
        .build();
    let list = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .margin_start(8)
        .margin_end(8)
        .build();

    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&list)
        .build();

    page.append(&banner);
    page.append(&empty);
    page.append(&scroll);

    let (sender, receiver) = crate::configuration::ui_channel::unbounded();
    let app = app_settings.clone();
    thread::spawn(move || {
        loop {
            let snapshot = app.lock().unwrap().clone();
            let mut rows = Vec::new();
            let mut offline = false;
            for wallet in &snapshot.btc_wallets {
                if nav::label_is_offline(&wallet.balance.lock().unwrap()) {
                    offline = true;
                }
                for item in wallet.history.lock().unwrap().iter() {
                    let sign = if item.amount_sats >= 0 { "+" } else { "-" };
                    let amount = if item.amount_sats >= 0 {
                        crate::currencies::btc_chain::format_btc(item.amount_sats as u64)
                    } else {
                        crate::currencies::btc_chain::format_btc(item.amount_sats.unsigned_abs())
                    };
                    let txid = &item.txid[..item.txid.len().min(12)];
                    rows.push((
                        item.amount_sats >= 0,
                        format!("{sign}{amount} BTC  {} conf  {txid}", item.confirmations),
                    ));
                }
            }
            for wallet in &snapshot.eth_wallets {
                if nav::label_is_offline(&wallet.balance.lock().unwrap()) {
                    offline = true;
                }
                for item in wallet.history.lock().unwrap().iter() {
                    let sign = if item.incoming { "+" } else { "-" };
                    let txid = if item.txid.len() > 12 {
                        &item.txid[..12]
                    } else {
                        &item.txid
                    };
                    rows.push((
                        item.incoming,
                        format!(
                            "{sign}{} {}  {} conf  {txid}",
                            item.amount, item.symbol, item.confirmations
                        ),
                    ));
                }
            }
            for wallet in &snapshot.sol_wallets {
                if nav::label_is_offline(&wallet.balance.lock().unwrap()) {
                    offline = true;
                }
                for item in wallet.history.lock().unwrap().iter() {
                    let sign = if item.incoming { "+" } else { "-" };
                    let txid = if item.txid.len() > 12 {
                        &item.txid[..12]
                    } else {
                        &item.txid
                    };
                    rows.push((
                        item.incoming,
                        format!(
                            "{sign}{} {}  {} conf  {txid}",
                            item.amount, item.symbol, item.confirmations
                        ),
                    ));
                }
            }
            for wallet in &snapshot.ltc_wallets {
                if nav::label_is_offline(&wallet.balance.lock().unwrap()) {
                    offline = true;
                }
                for item in wallet.history.lock().unwrap().iter() {
                    let sign = if item.amount_sats >= 0 { "+" } else { "-" };
                    let amount = if item.amount_sats >= 0 {
                        crate::currencies::ltc_chain::format_ltc(item.amount_sats as u64)
                    } else {
                        crate::currencies::ltc_chain::format_ltc(item.amount_sats.unsigned_abs())
                    };
                    let txid = &item.txid[..item.txid.len().min(12)];
                    rows.push((
                        item.amount_sats >= 0,
                        format!("{sign}{amount} LTC  {} conf  {txid}", item.confirmations),
                    ));
                }
            }
            if sender.send_blocking((rows, offline)).is_err() {
                break;
            }
            thread::sleep(Duration::from_secs(5));
        }
    });

    crate::configuration::ui_channel::attach(
        receiver,
        clone!(
            #[weak] list,
            #[weak] empty,
            #[weak] banner,
            #[upgrade_or]
            ControlFlow::Break,
            move |(rows, offline): (Vec<(bool, String)>, bool)| {
                while let Some(child) = list.first_child() {
                    list.remove(&child);
                }
                empty.set_visible(rows.is_empty());
                banner.set_label(if offline {
                    "A node is unreachable. Receive still works offline."
                } else {
                    "Activity is recent history from this device. Receive still works if a node is unreachable."
                });
                for (incoming, text) in rows {
                    let css = if incoming {
                        "transaction-positive"
                    } else {
                        "transaction-negative"
                    };
                    list.append(
                        &gtk::Label::builder()
                            .label(&text)
                            .wrap(true)
                            .halign(Align::Start)
                            .margin_top(4)
                            .margin_bottom(4)
                            .css_classes([css])
                            .build(),
                    );
                }
                ControlFlow::Continue
            }
        ),
    );

    page
}
