use gtk::prelude::*;
use gtk::{Orientation};
use glib::{clone, Continue, MainContext, PRIORITY_DEFAULT};
use std::thread;
use std::time::Duration;

use crate::currencies::currency_pairs::CurrencyPairs;

pub fn home_view(mut currency_pairs: CurrencyPairs) -> gtk::Box {
    let home_box = gtk::Box::new(Orientation::Vertical, 15);
    let btc_box = gtk::Box::new(Orientation::Horizontal, 185);

    let btc_label  = gtk::Label::builder()
        .label("BTC")
        .margin_top(12)
        .margin_start(50)
        .build();
    
    let btc_price_label  = gtk::Label::builder()
        .label("Uninitialized")
        .margin_top(12)
        .margin_end(50)
        .build();

    btc_box.append(&btc_label);
    btc_box.append(&btc_price_label);

    let eth_box = gtk::Box::new(Orientation::Horizontal, 185);
    let eth_label  = gtk::Label::builder()
        .label("ETH")
        .margin_top(12)
        .margin_start(50)
        .build();

    let eth_price_label  = gtk::Label::builder()
        .label("Uninitialized")
        .margin_top(12)
        .margin_end(50)
        .build();

    eth_box.append(&eth_label);
    eth_box.append(&eth_price_label);

    home_box.append(&btc_box);
    home_box.append(&eth_box);

    let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);

    thread::spawn(move || {
        loop {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let sender = sender.clone();
            let _ = runtime.block_on(runtime.spawn(async move {
                let btc_label = match CurrencyPairs::get_btc_price().await {
                    Ok(label)  => label,
                    Err(_) => String::from("Uninitialized")
                };

                let eth_label = match CurrencyPairs::get_eth_price().await {
                    Ok(label)  => label,
                    Err(_) => String::from("Uninitialized")
                };

                sender.send((btc_label, eth_label)).expect("Could not send through channel");
            }));
            thread::sleep(Duration::from_secs(60));
        }
    });

    receiver.attach(
        None,
        clone!(@weak home_box => @default-return Continue(false),
            move |price_text| {
                if price_text.0 != "Uninitialized" {
                    btc_price_label.set_label(&price_text.0);
                    currency_pairs.btc_usd = Some(price_text.0.to_string());
                }
                
                if price_text.1 != "Uninitialized" {
                    eth_price_label.set_label(&price_text.1);
                    currency_pairs.eth_usd = Some(price_text.1.to_string());
                }

                Continue(true)
            }
        ),
    );

    return home_box.clone();
}