use adw::prelude::*;
use adw::{ApplicationWindow, HeaderBar};
use glib::clone;
use std::sync::{Arc, Mutex};

use crate::configuration::application_settings::*;
use crate::views::ui;
use crate::views::{login, settings};

pub fn header_bar_view(window: ApplicationWindow, app_settings: Arc<Mutex<ApplicationSettings>>) -> adw::HeaderBar {
    let header_bar = HeaderBar::new();

    // Title block: app name plus the live/test chip. Which network you are on decides
    // whether a send moves real money, so it is worth a permanent slot rather than a trip
    // into Settings to check.
    let title_box = ui::vbox(0);
    title_box.set_halign(gtk::Align::Center);
    title_box.append(
        &gtk::Label::builder()
            .label("Block Wallet")
            .css_classes(["title-4"])
            .build(),
    );

    let is_test = app_settings.lock().unwrap().is_test_mode();
    let chip = ui::network_chip(if is_test { "TEST NETWORKS" } else { "LIVE" }, is_test);
    chip.set_halign(gtk::Align::Center);
    title_box.append(&chip);
    header_bar.set_title_widget(Some(&title_box));

    let settings_button = ui::flat_icon_button("preferences-system-symbolic", "Settings");
    let lock_button = ui::flat_icon_button("changes-prevent-symbolic", "Lock wallet");

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
