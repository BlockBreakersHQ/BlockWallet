use adw::prelude::*;
use adw::ApplicationWindow;
use glib::clone;
use gtk::prelude::*;
use gtk::{Button, Orientation};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::configuration::application_settings::*;
use crate::views::stack;

pub fn lock_and_show(window: ApplicationWindow, app_settings: Arc<Mutex<ApplicationSettings>>) {
    app_settings.lock().unwrap().lock_store();
    let snapshot = app_settings.lock().unwrap().clone();
    login_view(window, snapshot);
}

pub fn login_view(window: ApplicationWindow, app_settings: ApplicationSettings) {
    let title = gtk::Label::builder()
        .label("Unlock wallet")
        .css_classes(["title"])
        .build();
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&title));

    let logo_path = match ApplicationSettings::find_images_path() {
        Ok(mut path) => {
            path.push("Logo.png");
            path
        }
        Err(_) => PathBuf::new(),
    };
    let login_logo = gtk::Image::from_file(logo_path);
    login_logo.set_pixel_size(96);

    let intro = gtk::Label::builder()
        .label("Enter your password to decrypt this wallet on this device.")
        .wrap(true)
        .wrap_mode(pango::WrapMode::WordChar)
        .justify(gtk::Justification::Center)
        .css_classes(["label-standard"])
        .margin_start(16)
        .margin_end(16)
        .build();

    let button = Button::builder()
        .label("Unlock")
        .margin_top(6)
        .margin_bottom(12)
        .margin_start(16)
        .margin_end(16)
        .hexpand(true)
        .build();
    button.add_css_class("standard_button");
    button.add_css_class("suggested-action");

    let input = gtk::Entry::builder()
        .placeholder_text("Password")
        .margin_top(12)
        .margin_bottom(6)
        .margin_start(16)
        .margin_end(16)
        .visibility(false)
        .hexpand(true)
        .build();

    let failed_login = gtk::Label::builder()
        .label("Unlock failed. Check the password and try again.")
        .margin_start(16)
        .margin_end(16)
        .wrap(true)
        .wrap_mode(pango::WrapMode::WordChar)
        .visible(false)
        .css_classes(["label-error"])
        .build();

    let page = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_start(16)
        .margin_end(16)
        .margin_top(12)
        .margin_bottom(16)
        .spacing(10)
        .build();
    page.append(&login_logo);
    page.append(&intro);
    page.append(&input);
    page.append(&failed_login);
    page.append(&button);

    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(400);
    clamp.set_child(Some(&page));

    let login_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();
    login_box.append(&header);
    login_box.append(&clamp);

    window.set_content(Some(&login_box));
    window.present();
    let app_settings = Arc::new(Mutex::new(app_settings));

    button.connect_clicked(clone!(
        #[weak] window,
        #[weak] input,
        #[weak] failed_login,
        #[strong] app_settings,
        move |_| {
            try_unlock(&window, &input, &failed_login, &app_settings);
        }
    ));
    input.connect_activate(clone!(
        #[weak] window,
        #[weak] input,
        #[weak] failed_login,
        #[strong] app_settings,
        move |_| {
            try_unlock(&window, &input, &failed_login, &app_settings);
        }
    ));
}

fn try_unlock(
    window: &ApplicationWindow,
    input: &gtk::Entry,
    failed_login: &gtk::Label,
    app_settings: &Arc<Mutex<ApplicationSettings>>,
) {
    let password = input.text().to_string();
    input.set_text("");
    if password.is_empty() {
        failed_login.set_visible(true);
        return;
    }

    let unlocked = match app_settings.lock().unwrap().unlock_store(&password) {
        Ok(_) => true,
        Err(_) => {
            failed_login.set_visible(true);
            false
        }
    };

    if unlocked {
        failed_login.set_visible(false);
        let settings = app_settings.lock().unwrap().clone();
        stack::stack_view(window, settings);
    }
}
