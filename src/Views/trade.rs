use gtk::prelude::*;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::ApplicationSettings;
use crate::currencies::trade::*;
use crate::currencies::tokens::Tokens;
use crate::Token;

pub fn trade_view(app_settings: Arc<Mutex<ApplicationSettings>>) -> (gtk::Box, Arc<Mutex<ApplicationSettings>>) {
    let trade_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();

    let token_list = token_search_builder(app_settings.lock().unwrap().tokens.clone());
    
    let app_settings_clone = app_settings.clone();

    let trade_label  = gtk::Label::builder()
        .label("Token Swap")
        .margin_top(5)
        .margin_start(5)
        .build();

    let token_list_clone = app_settings.lock().unwrap().tokens.clone();
    
    thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let _ = runtime.block_on(runtime.spawn(async move {
            let mut eth_token = Token::empty();
            let mut usdc_token = Token::empty();
            for token in token_list_clone.eth_tokens {
                if token.1.symbol == "ETH" {
                    eth_token = token.1;
                } else if token.1.symbol == "USDC" {
                    usdc_token = token.1;
                }
            }
            let trade = Trade::new(app_settings_clone.lock().unwrap().clone());
            trade.get_quote(usdc_token, eth_token, 1950.9).await;
        }));
    });

    let from_token = gtk::Entry::builder()
        .placeholder_text("Select Token")
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .hexpand(true)
        .halign(gtk::Align::Start)
        .build();

    let from_amount = gtk::Entry::builder()
        .placeholder_text("0.0")
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .build();

    let to_token = gtk::Entry::builder()
        .placeholder_text("Select Token")
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .hexpand(true)
        .halign(gtk::Align::Start)
        .build();

    let to_amount = gtk::Entry::builder()
        .placeholder_text("0.0")
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .build();

    let from_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .build();

    from_box.append(&from_token);
    from_box.append(&from_amount);

    let to_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .build();

    to_box.append(&to_token);
    to_box.append(&to_amount);

    let token_filter_model = gtk::FilterListModel::new(Some(&token_list), None::<&gtk::Filter>);
    let token_list = gtk::ListBox::new();
    //token_list.append(&token_filter_model);

    let swap_button = gtk::Button::builder()
        .label("Swap")
        .margin_top(6)
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(6)
        .build();
    
    trade_box.append(&trade_label);
    trade_box.append(&from_box);
    trade_box.append(&to_box);
    trade_box.append(&swap_button);
    trade_box.append(&token_list);
    return (trade_box, app_settings)
}

pub fn token_search_builder(tokens: Tokens) -> gio::ListStore {
    let token_list = gio::ListStore::new(glib::BoxedAnyObject::static_type());
    for (key, value) in tokens.eth_tokens.clone() {
        token_list.append(&glib::BoxedAnyObject::new(value.clone()));
    }

    for token in &token_list {
        //println!("token: {:?}", token.unwrap().downcast::<glib::BoxedAnyObject>().unwrap().borrow::<Token>().address);
    }

    token_list
}   