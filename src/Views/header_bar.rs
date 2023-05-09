use adw::{HeaderBar, ApplicationWindow};
use gtk::{Image, Button};
use gtk::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::configuration::application_settings::*;
use crate::views::settings;

pub fn header_bar_view(window: ApplicationWindow, app_settings: Arc<Mutex<ApplicationSettings>>) -> adw::HeaderBar {
    if app_settings.lock().unwrap().logged_in == false {
        let new_app_settings = ApplicationSettings::new(app_settings.lock().unwrap().tokens.clone());
        *app_settings.lock().unwrap() = new_app_settings;
    }

    let settings_icon_path = match ApplicationSettings::find_images_path(){
        Ok(mut lp) => {
            lp.push("cog.png");
            lp
        },
        Err(_) => PathBuf::new()
    };
    
    let header_bar = HeaderBar::new();
    //header_bar.set_show_end_title_buttons(false);
    let settings_button = Button::new();
    
    let settings_icon = Image::from_file(settings_icon_path);
    settings_icon.set_pixel_size(25);
    settings_button.set_child(Some(&settings_icon));

    header_bar.pack_end(&settings_button);

    settings_button.connect_clicked(move |_| {
        settings::settings_view(window.clone(), app_settings.clone());
    });

    return header_bar;
}