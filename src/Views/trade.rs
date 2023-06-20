use gtk::prelude::*;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::ApplicationSettings;
use crate::currencies::trade::*;
use crate::currencies::tokens::Tokens;
use crate::Token;

pub fn trade_view(app_settings: Arc<Mutex<ApplicationSettings>>) -> (gtk::Box, Arc<Mutex<ApplicationSettings>>) {
    let trade_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();

    let mut token_list = Vec::<&str>::new();
    let tokens = app_settings.lock().unwrap().tokens.eth_tokens.clone();
    for (key, value) in &tokens {
        token_list.push(key);
    }
    
    let app_settings_clone = app_settings.clone();

    let trade_label  = gtk::Label::builder()
        .label("Token Swap")
        .margin_top(5)
        .margin_start(5)
        .build();

    

    let from_token = gtk::DropDown::from_strings(&token_list.as_slice());
    from_token.set_margin_top(6);
    from_token.set_margin_bottom(6);
    from_token.set_margin_start(12);
    from_token.set_margin_end(12);
    from_token.set_hexpand(true);
    from_token.set_enable_search(true);
    from_token.set_halign(gtk::Align::Start);

    let from_amount = gtk::Entry::builder()
        .placeholder_text("0.0")
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .build();

    let to_token = gtk::DropDown::from_strings(&token_list.as_slice());
    to_token.set_margin_top(6);
    to_token.set_margin_bottom(6);
    to_token.set_margin_start(12);
    to_token.set_margin_end(12);
    to_token.set_hexpand(true);
    to_token.set_enable_search(true);
    to_token.set_halign(gtk::Align::Start);
    //to_token.set_expression(Some(token_list));

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

    //let token_filter_model = gtk::FilterListModel::new(Some(&token_list), None::<&gtk::Filter>);
    //let token_list = gtk::ListBox::new();
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

    let token_list_clone = app_settings.lock().unwrap().tokens.clone();

    let trade = Trade::new(app_settings.lock().unwrap().clone());
    
    let mut from_trade = Arc::new(Mutex::new(trade.clone()));
    let from_app_sett = app_settings.clone();

    let mut to_trade = Arc::new(Mutex::new(trade.clone()));
    let to_app_sett = app_settings.clone();
    
    from_token.connect_selected_notify(move |dropdown| {
        let ticker = String::from(gtk::StringObject::from(dropdown.selected_item().unwrap().downcast::<gtk::StringObject>().unwrap()).string().as_str());
        let from_ticker = from_app_sett.lock().unwrap().tokens.eth_tokens[&ticker].clone();
        from_trade.lock().unwrap().from_token = from_ticker;
    });

    to_token.connect_selected_notify(move |dropdown| {
        let ticker = String::from(gtk::StringObject::from(dropdown.selected_item().unwrap().downcast::<gtk::StringObject>().unwrap()).string().as_str());
        let to_ticker = to_app_sett.lock().unwrap().tokens.eth_tokens[&ticker].clone();
        to_trade.lock().unwrap().to_token = to_ticker;
    });

    thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let _ = runtime.block_on(runtime.spawn(async move {
            loop {
                //let mut trade = Trade::new(app_settings_clone.lock().unwrap().clone());
                //trade.from_amount = 1700.0;
                //let trade = trade.get_quote().await;
                //thread::sleep(Duration::from_secs(5));
                //println!("to amount: {:?}", trade.clone().to_amount);
            }
        }));
    });

    return (trade_box, app_settings)
}