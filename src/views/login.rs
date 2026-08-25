use adw::prelude::*;
use adw::ApplicationWindow;
use glib::clone;
use gtk::Orientation;
use std::sync::{Arc, Mutex};

use crate::configuration::application_settings::*;
use crate::views::stack;
use crate::views::ui;

pub fn lock_and_show(window: ApplicationWindow, app_settings: Arc<Mutex<ApplicationSettings>>) {
    app_settings.lock().unwrap().lock_store();
    let snapshot = app_settings.lock().unwrap().clone();
    login_view(window, snapshot);
}

pub fn login_view(window: ApplicationWindow, app_settings: ApplicationSettings) {
    let header = adw::HeaderBar::new();
    header.add_css_class("flat");
    header.set_title_widget(Some(&gtk::Label::new(None)));

    // The unlock screen is the app's first impression and has exactly one job, so it gets
    // a centred lockup rather than a form crammed against the top of the window.
    let lockup = ui::vbox(10);
    lockup.set_halign(gtk::Align::Center);
    let logo = ui::logo_image(190);
    let has_logo = logo.is_some();
    match &logo {
        Some(image) => lockup.append(image),
        None => {
            let fallback = gtk::Image::from_icon_name("changes-prevent-symbolic");
            fallback.set_pixel_size(88);
            lockup.append(&fallback);
        }
    }
    // The logo artwork already carries the "Block Wallet" wordmark, so the text title
    // would just say it twice. It stays as the fallback for when the file is missing and
    // only a symbolic icon is shown.
    if !has_logo {
        lockup.append(
            &gtk::Label::builder()
                .label("Block Wallet")
                .css_classes(["onboard-title"])
                .build(),
        );
    }
    lockup.append(
        &gtk::Label::builder()
            .label("Enter your password to decrypt this wallet on this device.")
            .wrap(true)
            .wrap_mode(pango::WrapMode::WordChar)
            .justify(gtk::Justification::Center)
            .max_width_chars(34)
            .css_classes(["onboard-subtitle"])
            .build(),
    );

    let form = adw::PreferencesGroup::new();
    let input = adw::PasswordEntryRow::builder().title("Password").build();
    form.add(&input);

    let button = ui::primary_button("Unlock");

    let failed_login = ui::error_label("Unlock failed. Check the password and try again.");
    failed_login.set_halign(gtk::Align::Center);

    let page = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .valign(gtk::Align::Center)
        .vexpand(true)
        .margin_start(ui::GUTTER)
        .margin_end(ui::GUTTER)
        .margin_top(ui::GUTTER)
        .margin_bottom(24)
        .spacing(20)
        .build();
    page.append(&lockup);
    page.append(&form);
    page.append(&failed_login);
    page.append(&button);

    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(400);
    clamp.set_vexpand(true);
    clamp.set_child(Some(&page));

    let login_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();
    login_box.append(&header);
    login_box.append(&ui::scroller(&clamp));

    window.set_content(Some(&ui::with_toasts(&login_box)));
    window.present();
    // Focus the field so unlocking is type-and-Enter, not tap-then-type.
    input.grab_focus();
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
    // `connect_entry_activated`, not `connect_activate`: the latter compiles, because
    // PasswordEntryRow inherits GtkListBoxRow's own "activate" signal, but it fires on row
    // activation rather than Enter in the text field — so pressing Enter would do nothing.
    input.connect_entry_activated(clone!(
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
    input: &adw::PasswordEntryRow,
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
