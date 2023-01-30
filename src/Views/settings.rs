use gtk::prelude::*;
use gtk::{Button, Orientation, Image};
use adw::{ApplicationWindow, HeaderBar};
use adw::prelude::*;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

use crate::configuration::application_settings::*;
use crate::views::{stack, login};

pub fn settings_view(window: ApplicationWindow, app_settings: ApplicationSettings) {
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

    let app_settings_logout = Arc::new(Mutex::new(app_settings.clone()));
    let window_logout       = window.clone();
    
    let label = gtk::Label::new(Some("this is a setting"));
    
    let button = Button::builder()
        .label("Submit")
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let logout_button = Button::builder()
        .label("Logout")
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let input = gtk::Entry::builder()
        .placeholder_text("setting 1")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .visibility(false)
        .build();

    let setting_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();
    setting_box.append(&header_bar);
    setting_box.append(&label);
    setting_box.append(&input);
    setting_box.append(&button);
    if app_settings.logged_in == true {
        setting_box.append(&logout_button);
    }

    window.show();
    window.set_content(Some(&setting_box));

    button.connect_clicked(move |button| {
        button.set_label("Clicked!");
    });

    logout_button.connect_clicked(move |_| {
        app_settings_logout.lock().unwrap().logged_in = false;
        login::login_view(window_logout.clone(), app_settings_logout.lock().unwrap().clone());
    });

    settings_button.connect_clicked(move |_| {
        if app_settings.logged_in == true {
            stack::stack_view(&window, app_settings.clone());
        }
        else {
            login::login_view(window.clone(), app_settings.clone());
        }
    });
}
