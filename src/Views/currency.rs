use gtk::prelude::*;
use gtk::{Orientation, Button, Align};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;
use glib::{clone, Continue, MainContext, PRIORITY_DEFAULT};

use crate::currencies::tokens::Token;
use crate::currencies::currency_pairs::*;
use crate::currencies::currency_pairs;
use crate::currencies::eth::EthereumWallet;
use crate::ApplicationSettings;
use crate::views::{transactions, home};

pub fn currency_view(token: Token, app_settings: ApplicationSettings) -> gtk::Box {
    let currency_box = gtk::Box::new(Orientation::Vertical, 0);
    let currency_price = Arc::new(Mutex::new(String::from("unintialized")));
    
    let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);
    let default = app_settings.default_currency.clone();
    let currency_token = token.clone();

    let transactions_box = get_transactions(token.clone(), app_settings.clone());

    thread::spawn(move || {
        loop {
            let default = default.clone();
            let currency_token = currency_token.clone();
            let sender = sender.clone();
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let _ = runtime.block_on(runtime.spawn(async move {
                let price_text = match CurrencyPairs::get_currency_price(currency_token, default).await {
                    Ok(price) => price,
                    Err(e) => String::from("unintialized")
                };

                match sender.send(price_text) {
                    Ok(_) => {},
                    Err(_) => {}
                };
                thread::sleep(Duration::from_secs(60));
            }));
        }
    });

    receiver.attach(
        None,
        clone!(@weak currency_price => @default-return Continue(false),
            move |price_text| {
                if price_text != "Uninitialized" {
                    *currency_price.lock().unwrap() = price_text;
                }
                Continue(true)
            }
        ),
    );

    let currency_label = home::generate_currency_box((token.clone(), currency_price));
    
    let transaction_detail_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .visible(false)
        .build();
    
    let currency_detail_box = gtk::Box::new(Orientation::Vertical, 0);
    let receive_box = generate_eth_receive_box(&app_settings.eth_wallets);

    let send_button = Button::builder()
        .label(&format!("Send {}", token.symbol))
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .build();
    send_button.add_css_class("standard_button");

    let receive_button = Button::builder()
        .label(&format!("Receive {}", token.symbol))
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .build();
    receive_button.add_css_class("standard_button");
    
    let scrollable_container = gtk::ScrolledWindow::builder()
        .child(&currency_detail_box)
        .vexpand(true)
        .build();
        
    currency_box.append(&scrollable_container);
    currency_box.append(&transaction_detail_box);
    currency_detail_box.append(&currency_label);
    currency_detail_box.append(&send_button);
    currency_detail_box.append(&receive_button);
    currency_detail_box.append(&receive_box);
    currency_detail_box.append(&transactions_box);

    send_button.connect_clicked(move |_| {
        transaction_detail_box.set_visible(true);
        scrollable_container.set_visible(false);
        transaction_detail_box.append(&transactions::transaction_view(app_settings.clone()).0);
    });

    receive_button.connect_clicked(move |_| {
        if receive_box.is_visible() == true {
            receive_box.set_visible(false);
        } else {
            receive_box.set_visible(true);
        }
    });
    
    return currency_box.clone();
}

pub fn get_balances_by_wallet(token: Token, app_settings: ApplicationSettings) -> gtk::Box {
    let wallet_balances_box = gtk::Box::new(Orientation::Vertical, 0);
    return wallet_balances_box;
}

pub fn get_transactions(token: Token, app_settings: ApplicationSettings) -> gtk::Box {
    let transactions_box = gtk::Box::new(Orientation::Vertical, 0);
    let mut transaction_counter = 0;
    
    let no_transactions_label = gtk::Label::builder()
        .label("No transactions to display.")
        .margin_top(5)
        .margin_start(5)
        .css_name("label-currency_name")
        .visible(false)
        .name("no_transactions_label")
        .build();

    transactions_box.append(&no_transactions_label);

    for ethw in app_settings.eth_wallets {
        let transactions = &*ethw.transactions.lock().unwrap();
        for transaction in transactions {
            let tokenSymbol = match &transaction.tokenSymbol {
                Some(symbol) => symbol,
                None => ""
            };
            if tokenSymbol == token.symbol {
                transaction_counter += 1;
                let transaction_box = gtk::Box::new(Orientation::Horizontal, 0);
                transaction_box.set_margin_bottom(5);
                let addresses_box = gtk::Box::new(Orientation::Vertical, 0);
                let amount_box = gtk::Box::new(Orientation::Vertical, 0);
                addresses_box.set_hexpand(true);

                let sender = match &transaction.from {
                    Some(sender) => format!("{}...", &sender[..25]),
                    None => "".to_string()
                };
                let receiver = match &transaction.to {
                    Some(receiver) => format!("{}...", &receiver[..25]),
                    None => "".to_string()
                };
                let decimals = match &transaction.tokenDecimal {
                    Some(decimals) => decimals.parse::<i32>().expect("ERROR: Parsing decimal failed.") + 1,
                    None => 0
                };
                let amount = match &transaction.value {
                    Some(amount) => {
                        let transaction_amount = amount.parse::<f64>().expect("ERROR: Parsing transaction amount failed.");
                        let transaction_value = transaction_amount / CurrencyPairs::get_exponent(decimals);
                        transaction_value
                    },
                    None => 0.0
                };

                let address = match ethw.address.clone() {
                    Some(address) => address.to_lowercase(),
                    None => "".to_string()
                };

                let sender_label = gtk::Label::builder()
                    .label(&format!("Sender:    {}", sender))
                    .halign(Align::Start)
                    .margin_top(5)
                    .margin_bottom(5)
                    .margin_start(5)
                    .margin_end(5)
                    .build();
                let reciever_label = gtk::Label::builder()
                    .label(&format!("Receiver: {}", receiver))
                    .halign(Align::Start)
                    .margin_top(5)
                    .margin_bottom(5)
                    .margin_start(5)
                    .margin_end(5)
                    .build();

                if receiver == address {
                    let amount_label = gtk::Label::builder()
                        .label(&format!("{}", amount))
                        .halign(Align::End)
                        .margin_top(5)
                        .margin_bottom(5)
                        .margin_start(5)
                        .margin_end(5)
                        .css_name("positive_transaction_label")
                        .build();
                    amount_box.append(&amount_label);
                } else {
                    let amount_label = gtk::Label::builder()
                        .label(&format!("{}", amount))
                        .halign(Align::End)
                        .margin_top(5)
                        .margin_bottom(5)
                        .margin_start(5)
                        .margin_end(5)
                        .css_name("negative_transaction_label")
                        .build();
                    amount_box.append(&amount_label);
                }

                addresses_box.append(&sender_label);
                addresses_box.append(&reciever_label);

                transaction_box.append(&addresses_box);
                transaction_box.append(&amount_box);
                transactions_box.append(&transaction_box);
                transaction_box.set_margin_start(5);
                transaction_box.set_margin_end(5);
            }
        }
    }

    if transaction_counter == 0 {
        no_transactions_label.set_visible(true);
    }
    return transactions_box;
}

pub fn generate_eth_receive_box(eth_wallets: &Vec<EthereumWallet>) -> gtk::Box {
    let receive_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .visible(false)
        .margin_top(5)
        .margin_bottom(5)
        .margin_start(12)
        .margin_end(12)
        .css_name("reciever_box")
        .name("reciever_box")
        .build();

    let receive_label = gtk::Label::builder()
        .label("Send to any of these addresses:")
        .build();
    
    receive_box.append(&receive_label);
    
    for ethw in eth_wallets {
        let address = match ethw.address.clone() {
            Some(address) => address,
            None => "".to_string()
        };

        let address_label = gtk::Label::builder()
            .label(&address)
            .selectable(true)
            .build();

        receive_box.append(&address_label);
    }

    return receive_box;
}
