use adw::prelude::*;
use adw::{ApplicationWindow, HeaderBar};
use glib::clone;
use gtk::prelude::*;
use gtk::{Button, Image};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::configuration::application_settings::*;
use crate::views::{login, settings};

pub fn header_bar_view(window: ApplicationWindow, app_settings: Arc<Mutex<ApplicationSettings>>) -> adw::HeaderBar {
    let settings_icon_path = match ApplicationSettings::find_images_path() {
        Ok(mut path) => {
            path.push("cog.png");
            path
        }
        Err(_) => PathBuf::new(),
    };

    let header_bar = HeaderBar::new();
    let settings_button = Button::new();
    let settings_icon = Image::from_file(settings_icon_path);
    settings_icon.set_pixel_size(25);
    settings_button.set_child(Some(&settings_icon));
    settings_button.set_tooltip_text(Some("Settings"));

    let lock_button = Button::builder().label("Lock").build();
    lock_button.add_css_class("standard_button");
    lock_button.set_valign(gtk::Align::Center);
    lock_button.set_tooltip_text(Some("Lock wallet"));

    header_bar.pack_end(&settings_button);
    header_bar.pack_end(&lock_button);

    settings_button.connect_clicked(clone!(
        #[weak] window,
        #[strong] app_settings,
        move |_| {
            settings::settings_view(window.clone(), app_settings.clone());
        }
    ));

    lock_button.connect_clicked(clone!(
        #[weak] window,
        #[strong] app_settings,
        move |_| {
            login::lock_and_show(window.clone(), app_settings.clone());
        }
    ));

    header_bar
}
