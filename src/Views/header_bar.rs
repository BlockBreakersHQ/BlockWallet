use adw::{HeaderBar, ApplicationWindow};
use gtk::{Image, Button};
use gtk::prelude::*;

use crate::configuration::application_settings::*;
use crate::views::settings;

pub fn header_bar_view(window: ApplicationWindow, mut app_settings: ApplicationSettings) -> adw::HeaderBar {
    if app_settings.logged_in == false {
        app_settings = ApplicationSettings::new();
    }
    
    let header_bar = HeaderBar::new();
    let settings_button = Button::new();
    
    let settings_icon = Image::from_file("cog.png");
    settings_icon.set_pixel_size(25);
    settings_button.set_child(Some(&settings_icon));

    header_bar.pack_start(&settings_button);

    settings_button.connect_clicked(move |_| {
        settings::settings_view(window.clone(), app_settings.clone());
    });

    return header_bar;
}