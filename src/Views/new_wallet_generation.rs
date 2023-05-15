use gtk::prelude::*;
use gtk::{Button, Orientation, Image};
use adw::prelude::*;
use adw::{ApplicationWindow};
use sha3::{Digest, Sha3_256};
use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use std::fs;

use crate::configuration::application_settings::*;
use crate::views::{login, stack, header_bar};

pub fn generate_wallet_view(window: ApplicationWindow, app_settings: ApplicationSettings) {
    let header_bar = header_bar::header_bar_view(window.clone(), Arc::new(Mutex::new(app_settings.clone())));

    let logo_path = match ApplicationSettings::find_images_path(){
        Ok(mut lp) => {
            lp.push("Logo.png");
            lp
        },
        Err(_) => PathBuf::new()
    };

    let login_logo = Image::from_file(logo_path);
    login_logo.set_pixel_size(300);

    let new_wallet_notice = gtk::Label::builder()
        .label("No wallet file has been provided. Please enter a password for a new wallet to be generated.")
        .margin_top(5)
        .margin_start(5)
        .margin_end(5)
        .wrap(true)
        .wrap_mode(pango::WrapMode::Char)
        .max_width_chars(50)
        .css_name("label-error")
        .build();

    let generate_wallet_button = Button::builder()
        .label("Generate Wallet")
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    generate_wallet_button.add_css_class("standard_button");

    let password_input = gtk::Entry::builder()
        .placeholder_text("Password")
        .margin_top(12)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .visibility(false)
        .build();

    let repeat_password_input = gtk::Entry::builder()
        .placeholder_text("Repeat password")
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .visibility(false)
        .build();

    let passwords_no_match = gtk::Label::builder()
        .label("Passwords do not match.")
        .margin_top(5)
        .margin_start(5)
        .visible(false)
        .css_name("label-error")
        .build();
        
    let import_message = gtk::Label::builder()
        .label("Import wallet from file path")
        .margin_top(5)
        .margin_start(5)
        .css_name("label-standard")
        .build();

    let path_input = gtk::Entry::builder()
        .placeholder_text("Path")
        .margin_top(12)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .visibility(false)
        .build();

    let import_wallet_button = Button::builder()
        .label("Import")
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    import_wallet_button.add_css_class("standard_button");

    let invalid_path = gtk::Label::builder()
        .label("File not found or incorrect type.")
        .margin_top(5)
        .margin_start(5)
        .visible(false)
        .css_name("label-error")
        .build();

    let password_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();
    
    password_box.append(&header_bar);
    password_box.append(&login_logo);
    password_box.append(&new_wallet_notice);
    password_box.append(&password_input);
    password_box.append(&repeat_password_input);
    password_box.append(&passwords_no_match);
    password_box.append(&generate_wallet_button);
    password_box.append(&import_message);
    password_box.append(&path_input);
    password_box.append(&import_wallet_button);
    password_box.append(&invalid_path);

    let hash = Arc::new(Mutex::new(String::new()));

    window.show();
    window.set_content(Some(&password_box));
    let window_import = window.clone();
    let app_settings_generate = Arc::new(Mutex::new(app_settings.clone()));
    let app_settings_import = Arc::new(Mutex::new(app_settings.clone()));

    generate_wallet_button.connect_clicked(move |_| {
        let mut logged_in = false;
        let mut hash = hash.lock().unwrap();
        let mut hasher = Sha3_256::new();

        if password_input.text() != repeat_password_input.text() {
            passwords_no_match.set_visible(true);
            password_input.set_text("");
            repeat_password_input.set_text("");
        } else {
            hasher.update(password_input.text());
            *hash = format!("{:X}", hasher.finalize());
            password_input.set_text("");
            repeat_password_input.set_text("");
    
            app_settings_generate.lock().unwrap().user_hash = hash.clone();
    
            match app_settings_generate.lock().unwrap().read_config() {
                Ok(_) => logged_in = true,
                Err(_) => passwords_no_match.set_visible(true)
            };
    
            if logged_in == true {
                let app_settings_pass = app_settings_generate.lock().unwrap().clone();
                stack::stack_view(&window, app_settings_pass);
            }
        }
    });

    import_wallet_button.connect_clicked(move |_| {
        if Path::new(&path_input.text()).exists() && path_input.text().contains(".dic") {
            if app_settings_import.lock().unwrap().config_path.clone().exists() {
                let mut new_path = app_settings_import.lock().unwrap().config_path.clone();
                new_path.pop();
                new_path.push(format!("Config-Old-{}.dic", chrono::offset::Local::now()));
                let _ = fs::rename(&app_settings_import.lock().unwrap().config_path, new_path);
            }
            match fs::copy(path_input.text(), app_settings_import.lock().unwrap().config_path.clone()) {
                Ok(_) => println!("Copied config.dic to executable directory."),
                Err(e) => println!("ERROR: Could not copy config.dic to executable directory. Error: {:?}", e)
            };
            path_input.set_text("");
            login::login_view(window_import.clone(), app_settings_import.lock().unwrap().clone());
        } else {
            invalid_path.set_visible(true);
            path_input.set_text("");
        }
    });
}