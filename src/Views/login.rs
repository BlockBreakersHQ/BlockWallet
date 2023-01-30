use gtk::prelude::*;
use gtk::{Button, Orientation, Image};
use adw::prelude::*;
use adw::{ApplicationWindow};
use sha3::{Digest, Sha3_256};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

use crate::configuration::application_settings::*;
use crate::views::{stack, header_bar};

pub fn login_view(window: ApplicationWindow, app_settings: ApplicationSettings) {
    let header_bar = header_bar::header_bar_view(window.clone(), app_settings.clone());

    let logo_path = match ApplicationSettings::find_images_path(){
        Ok(mut lp) => {
            lp.push("Logo.png");
            lp
        },
        Err(_) => PathBuf::new()
    };

    let login_logo = Image::from_file(logo_path);
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

    let hash = Arc::new(Mutex::new(String::new()));

    window.show();
    window.set_content(Some(&login_box));
    let app_settings_clone = Arc::new(Mutex::new(app_settings.clone()));

    button.connect_clicked(move |_| {
        let mut logged_in = false;
        let mut hash = hash.lock().unwrap();
        let mut hasher = Sha3_256::new();
        hasher.update(input.text());
        *hash = format!("{:X}", hasher.finalize());
        input.set_text("");

        app_settings_clone.lock().unwrap().user_hash = hash.clone();

        match app_settings_clone.lock().unwrap().read_config() {
            Ok(_) => logged_in = true,
            Err(_) => failed_login.set_visible(true)
        };

        if logged_in == true {
            let app_settings_pass = app_settings_clone.lock().unwrap().clone();
            stack::stack_view(&window, app_settings_pass);
        }
    });
}
