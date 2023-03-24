use std::thread;
use gtk::prelude::*;
use gtk::{Orientation, Button};

use crate::ApplicationSettings;
use crate::currencies::tokens::Token;

pub fn transaction_view(app_settings: ApplicationSettings, token: Token) -> (gtk::Box, ApplicationSettings) {
    let transaction_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();

    let mut wallet_names = Vec::<&str>::new();
    for wallet in &app_settings.eth_wallets {
        let name = match &wallet.wallet_name {
            Some(name) => name,
            None => "Unnamed Wallet",
        };
        wallet_names.push(&name);
    }

    let send_address = gtk::DropDown::from_strings(&wallet_names);
    send_address.set_margin_top(12);
    send_address.set_margin_bottom(6);
    send_address.set_margin_start(12);
    send_address.set_margin_end(12);
        //.build();

    let receive_address = gtk::Entry::builder()
        .placeholder_text("Recieve Address")
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .build();

    let amount = gtk::Entry::builder()
        .placeholder_text("Amount")
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .build();
    
    let advanced_button = Button::builder()
        .label("Show Advanced")
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .build();

    let gas = gtk::Entry::builder()
        .placeholder_text("Gas (Optional)")
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .visible(false)
        .build();

    let gas_price = gtk::Entry::builder()
        .placeholder_text("Gas Price (Optional)")
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .visible(false)
        .build();

    let data = gtk::Entry::builder()
        .placeholder_text("Data (Optional)")
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .visible(false)
        .build();

    let submit_button = Button::builder()
        .label("Submit")
        .margin_top(6)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    
    submit_button.add_css_class("transaction_button");

    let transaction_error = gtk::Label::builder()
        .label("Receive address is required.")
        .margin_top(5)
        .margin_start(5)
        .visible(false)
        .css_name("label-error")
        .build();

    transaction_box.append(&send_address);
    transaction_box.append(&receive_address);
    transaction_box.append(&amount);
    transaction_box.append(&advanced_button);
    transaction_box.append(&gas);
    transaction_box.append(&gas_price);
    transaction_box.append(&data);
    transaction_box.append(&submit_button);
    transaction_box.append(&transaction_error);

    let app_settings_clone = app_settings.clone();

    advanced_button.connect_clicked(move |advanced_button| {
        if advanced_button.label() == Some("Hide Advanced".into()) {
            advanced_button.set_label("Show Advanced");
            gas.set_visible(false);
            gas_price.set_visible(false);
            data.set_visible(false);
        } else {
            advanced_button.set_label("Hide Advanced");
            gas.set_visible(true);
            gas_price.set_visible(true);
            data.set_visible(true);
        }
    });

    submit_button.connect_clicked(move |_| {
        if amount.text().to_string() == "" {
            transaction_error.set_label("Amount must be provided.");
            transaction_error.set_visible(true);
        } else if receive_address.text().to_string() == "" {
            transaction_error.set_label("Receive address is required.");
            transaction_error.set_visible(true);
        }
        else if token.symbol == "ETH" {
            transaction_error.set_visible(false);
            let receiver = receive_address.text().to_string();
            let amount = match amount.text().to_string().parse::<u64>() {
                Ok(amount) => {
                    if amount <= 0 {
                        transaction_error.set_label("Amount must be greater than 0.");
                        transaction_error.set_visible(true);
                    }
                    amount
                },
                Err(_) => {
                    transaction_error.set_label("Amount must be a number.");
                    transaction_error.set_visible(true);
                    0
                }
            };
            
            let wallet_num: usize = send_address.selected() as usize;
            let app_settings = app_settings_clone.clone();

            if amount > 0 && receiver != "" {
                thread::spawn(move || {
                    let app_settings = app_settings.clone();
                    let runtime = tokio::runtime::Runtime::new().unwrap();
                    let _ = runtime.block_on(runtime.spawn(async move {
                        match app_settings.eth_wallets[wallet_num].ether_transaction(&receiver, amount).await {
                            Ok(_) => (),
                            Err(err) => {
                                //transaction_error.set_label(&format!("{:?}", err));
                                app_settings.write_error(format!("{:?}", err));
                                ()
                            }
                        };
                    }));
                });
            }
            
        } else if token.symbol == "BTC" {
            println!("BTC Transaction");
        } else {
            println!("ERC20 Transaction");
        }
        println!("Submit button clicked");
    });

    return (transaction_box, app_settings);
}