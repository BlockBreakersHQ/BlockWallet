use gtk::prelude::*;
use gtk::{Button, Orientation, Image};
use adw::prelude::*;
use adw::{ApplicationWindow, HeaderBar};
use sha3::{Digest, Sha3_256};
use std::sync::{Arc, Mutex};

use crate::configuration::ApplicationSettings;
use crate::views::{home, settings};

pub fn login_view(window: ApplicationWindow, app_settings: ApplicationSettings) {
    if app_settings.logged_in == false {
        let app_settings = ApplicationSettings::new();
    }
    
    let header_bar = HeaderBar::new();
    let settings_button = Button::new();
    
    let settings_icon = Image::from_file("cog.png");
    settings_icon.set_pixel_size(25);
    settings_button.set_child(Some(&settings_icon));

    header_bar.pack_start(&settings_button);

    let login_logo = Image::from_file("Logo.png");
    login_logo.set_pixel_size(300);

    let button = Button::builder()
        .label("Submit")
        .margin_top(6)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    
    button.add_css_class("login_button");

    let input = gtk::Entry::builder()
        .placeholder_text("password")
        .margin_top(12)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .visibility(false)
        .build();

    let failed_login = gtk::Label::new(Some("Login attempt failed."));
    failed_login.set_visible(false);
    failed_login.add_css_class("failed_login");

    let login_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();
    login_box.append(&header_bar);
    login_box.append(&login_logo);
    login_box.append(&input);
    login_box.append(&failed_login);
    login_box.append(&button);

    let hash       = Arc::new(Mutex::new(String::new()));

    window.show();
    window.set_content(Some(&login_box));
    let window_clone = window.clone();
    let app_settings_clone = Arc::new(Mutex::new(app_settings.clone()));

    button.connect_clicked(move |_| {
        let mut logged_in = false;
        let mut hash = hash.lock().unwrap();
        let mut hasher = Sha3_256::new();
        hasher.update(input.text());
        *hash = String::from(format!("{:X}", hasher.finalize()));
        input.set_text("");

        app_settings_clone.lock().unwrap().user_hash = hash.clone();

        match app_settings_clone.lock().unwrap().read_config() {
            Ok(_) => logged_in = true,
            Err(_) => failed_login.set_visible(true)
        };

        if logged_in == true {
            let app_settings_pass = app_settings_clone.lock().unwrap().clone();
            home::home_view(&window, app_settings_pass);
        }
    });

    settings_button.connect_clicked(move |_| {
        settings::settings_view(window_clone.clone(), app_settings.clone());
    });
}
