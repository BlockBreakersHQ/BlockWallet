use gtk::prelude::*;
use gtk::{Button, Orientation, Image};
use adw::{ApplicationWindow, HeaderBar};
use adw::prelude::*;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

use crate::configuration::application_settings::*;
use crate::views::{stack, login};

pub fn settings_view(window: ApplicationWindow, app_settings: Arc<Mutex<ApplicationSettings>>) {
    let header_bar = HeaderBar::new();
    let settings_button = Button::new();

    let cog_path = match ApplicationSettings::find_images_path(){
        Ok(mut cog) => {
            cog.push("cog.png");
            cog
        },
        Err(_) => PathBuf::new()
    };
    
    let settings_icon = Image::from_file(cog_path);
    settings_icon.set_pixel_size(25);
    settings_button.set_child(Some(&settings_icon));

    header_bar.pack_start(&settings_button);

    let app_settings_logout = app_settings.clone();
    let window_logout       = window.clone();
    
    let label = gtk::Label::new(Some("this is a setting"));

    let input = gtk::Entry::builder()
        .placeholder_text("setting 1")
        .margin_top(12)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .visibility(false)
        .build();
    
    let submit_button = Button::builder()
        .label("Submit")
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .build();
        
    let network_settings_box = network_settings_box(app_settings.clone());

    let network_button = Button::builder()
        .label("Network Settings")
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .build();

    let logout_button = Button::builder()
    .label("Logout")
    .margin_top(6)
    .margin_bottom(6)
    .margin_start(12)
    .margin_end(12)
    .build();

    let setting_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();

    setting_box.append(&header_bar);
    setting_box.append(&label);
    setting_box.append(&input);
    setting_box.append(&submit_button);
    setting_box.append(&network_settings_box);

    if app_settings.lock().unwrap().logged_in == true {
        setting_box.append(&network_button);
        setting_box.append(&logout_button);
    }

    window.show();
    window.set_content(Some(&setting_box));

    submit_button.connect_clicked(move |button| {
        button.set_label("Clicked!");
    });

    network_button.connect_clicked(move |network_button| {
        if network_button.label() == Some("Network Settings".into()) {
            network_button.set_label("Hide Network Settings");
        }
        else {
            network_button.set_label("Network Settings");
        }
        if network_settings_box.get_visible() == true {
            network_settings_box.set_visible(false);
        }
        else {
            network_settings_box.set_visible(true);
        }
    });

    logout_button.connect_clicked(move |_| {
        app_settings_logout.lock().unwrap().logged_in = false;
        login::login_view(window_logout.clone(), app_settings_logout.lock().unwrap().clone());
    });

    settings_button.connect_clicked(move |_| {
        if app_settings.lock().unwrap().logged_in == true {
            stack::stack_view(&window, app_settings.lock().unwrap().clone());
        }
        else {
            login::login_view(window.clone(), app_settings.lock().unwrap().clone());
        }
    });
}

pub fn network_settings_box(app_settings: Arc<Mutex<ApplicationSettings>>) -> gtk::Box {
    let network_settings_box = gtk::Box::new(Orientation::Vertical, 0);
    network_settings_box.set_visible(false);

    let etherscan_api_key = gtk::Entry::builder()
        .placeholder_text("Etherscan Api Key")
        .margin_top(12)
        .margin_bottom(3)
        .margin_start(12)
        .margin_end(12)
        .build();

    let infura_key = gtk::Entry::builder()
        .placeholder_text("Infura API Key")
        .margin_top(3)
        .margin_bottom(3)
        .margin_start(12)
        .margin_end(12)
        .build();
    
    let ethereum_node = gtk::Entry::builder()
        .placeholder_text("Ethereum Node")
        .margin_top(3)
        .margin_bottom(3)
        .margin_start(12)
        .margin_end(12)
        .build();
    
    let eth_incorrect_format = gtk::Label::builder()
        .label("ETH Node format: protocol://ip:port")
        .visible(false)
        .css_name("label-error")
        .build();

    let btc_node = gtk::Entry::builder()
        .placeholder_text("Bitcoin Node")
        .margin_top(3)
        .margin_bottom(3)
        .margin_start(12)
        .margin_end(12)
        .build();

    let btc_incorrect_format = gtk::Label::builder()
        .label("BTC Node format: protocol://user:pass@ip:port")
        .visible(false)
        .css_name("label-error")
        .build();

    let save_button = Button::builder()
        .label("Save Settings")
        .margin_top(3)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    
    network_settings_box.append(&etherscan_api_key);
    network_settings_box.append(&infura_key);
    network_settings_box.append(&ethereum_node);
    network_settings_box.append(&eth_incorrect_format);
    network_settings_box.append(&btc_node);
    network_settings_box.append(&btc_incorrect_format);
    network_settings_box.append(&save_button);

    save_button.connect_clicked(move |_| {
        eth_incorrect_format.set_visible(false);
        btc_incorrect_format.set_visible(false);
        if !etherscan_api_key.text().is_empty() {
            app_settings.lock().unwrap().etherscan_key = etherscan_api_key.text().to_string();
        }
        if !ethereum_node.text().is_empty() {
            let slash_count = btc_node.text().matches("/").count();
            let co_count = btc_node.text().matches(":").count();
            if slash_count != 2 || co_count != 2 {
                eth_incorrect_format.set_visible(true);
            } else {
                app_settings.lock().unwrap().eth_node = ethereum_node.text().to_string();
            }
        }
        if !btc_node.text().is_empty() {
            let co_count = btc_node.text().matches(":").count();
            let at_count = btc_node.text().matches("@").count();
            if co_count != 3 || at_count != 1 {
                btc_incorrect_format.set_visible(true);
            } else {
                app_settings.lock().unwrap().btc_node = btc_node.text().to_string();
            }
        }
        if !infura_key.text().is_empty() {
            app_settings.lock().unwrap().infura_key = infura_key.text().to_string();
        }
    });
    
    return network_settings_box;
}