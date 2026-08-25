use adw::prelude::*;
use adw::ApplicationWindow;
use glib::clone;
use gtk::prelude::*;
use gtk::{Button, Orientation};
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::configuration::application_settings::*;
use crate::configuration::onboarding::{self, WordCount};
use crate::views::{login, stack};

#[derive(Default)]
struct Draft {
    mnemonic: String,
    passphrase: String,
    from_restore: bool,
}

pub fn generate_wallet_view(window: ApplicationWindow, app_settings: ApplicationSettings) {
    let app_settings = Arc::new(Mutex::new(app_settings));
    let draft = Rc::new(RefCell::new(Draft::default()));

    let title = gtk::Label::builder()
        .label("Set up wallet")
        .css_classes(["title"])
        .build();
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&title));

    let stack = gtk::Stack::builder()
        .vexpand(true)
        .hexpand(true)
        .transition_type(gtk::StackTransitionType::SlideLeftRight)
        .build();

    let (choice_page, create_12, create_24, restore_btn, import_btn, choice_error) = choice_page();
    let (show_page, seed_grid, seed_continue, seed_back) = show_seed_page();
    let (confirm_page, confirm_view, confirm_continue, confirm_back, confirm_error) = confirm_page();
    let (restore_page, restore_view, restore_pass, restore_continue, restore_back, restore_error) =
        restore_page();
    let (password_page, password_input, repeat_input, password_save, password_back, password_error) =
        password_page();
    let (import_page, path_input, import_save, import_back, import_error) = import_page();

    stack.add_named(&choice_page, Some("choice"));
    stack.add_named(&show_page, Some("show_seed"));
    stack.add_named(&confirm_page, Some("confirm"));
    stack.add_named(&restore_page, Some("restore"));
    stack.add_named(&password_page, Some("password"));
    stack.add_named(&import_page, Some("import"));
    stack.set_visible_child_name("choice");

    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(400);
    clamp.set_child(Some(&stack));

    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&clamp)
        .build();

    let outer = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();
    outer.append(&header);
    outer.append(&scroll);

    window.set_content(Some(&outer));
    window.present();

    create_12.connect_clicked(clone!(
        #[weak] stack,
        #[weak] title,
        #[weak] seed_grid,
        #[weak] choice_error,
        #[strong] draft,
        move |_| {
            start_create(
                WordCount::Words12,
                &draft,
                &seed_grid,
                &stack,
                &title,
                &choice_error,
            );
        }
    ));
    create_24.connect_clicked(clone!(
        #[weak] stack,
        #[weak] title,
        #[weak] seed_grid,
        #[weak] choice_error,
        #[strong] draft,
        move |_| {
            start_create(
                WordCount::Words24,
                &draft,
                &seed_grid,
                &stack,
                &title,
                &choice_error,
            );
        }
    ));
    restore_btn.connect_clicked(clone!(
        #[weak] stack,
        #[weak] title,
        #[weak] restore_error,
        move |_| {
            restore_error.set_visible(false);
            title.set_label("Restore wallet");
            stack.set_visible_child_name("restore");
        }
    ));
    import_btn.connect_clicked(clone!(
        #[weak] stack,
        #[weak] title,
        #[weak] import_error,
        move |_| {
            import_error.set_visible(false);
            title.set_label("Import wallet file");
            stack.set_visible_child_name("import");
        }
    ));

    seed_back.connect_clicked(clone!(
        #[weak] stack,
        #[weak] title,
        #[strong] draft,
        move |_| {
            *draft.borrow_mut() = Draft::default();
            title.set_label("Set up wallet");
            stack.set_visible_child_name("choice");
        }
    ));
    seed_continue.connect_clicked(clone!(
        #[weak] stack,
        #[weak] title,
        #[weak] confirm_error,
        #[weak] confirm_view,
        move |_| {
            confirm_error.set_visible(false);
            confirm_view.buffer().set_text("");
            title.set_label("Confirm phrase");
            stack.set_visible_child_name("confirm");
        }
    ));

    confirm_back.connect_clicked(clone!(
        #[weak] stack,
        #[weak] title,
        move |_| {
            title.set_label("Recovery phrase");
            stack.set_visible_child_name("show_seed");
        }
    ));
    confirm_continue.connect_clicked(clone!(
        #[weak] stack,
        #[weak] title,
        #[weak] confirm_error,
        #[weak] confirm_view,
        #[strong] draft,
        move |_| {
            let typed = textview_content(&confirm_view);
            match onboarding::confirm_created_phrase(&draft.borrow().mnemonic, &typed) {
                Ok(_) => {
                    confirm_error.set_visible(false);
                    title.set_label("Set password");
                    stack.set_visible_child_name("password");
                }
                Err(err) => {
                    confirm_error.set_label(err.as_label());
                    confirm_error.set_visible(true);
                }
            }
        }
    ));

    restore_back.connect_clicked(clone!(
        #[weak] stack,
        #[weak] title,
        #[strong] draft,
        move |_| {
            *draft.borrow_mut() = Draft::default();
            title.set_label("Set up wallet");
            stack.set_visible_child_name("choice");
        }
    ));
    restore_continue.connect_clicked(clone!(
        #[weak] stack,
        #[weak] title,
        #[weak] restore_error,
        #[weak] restore_view,
        #[weak] restore_pass,
        #[strong] draft,
        move |_| {
            match onboarding::parse_restore_phrase(&textview_content(&restore_view)) {
                Ok(phrase) => {
                    restore_error.set_visible(false);
                    let mut draft = draft.borrow_mut();
                    draft.mnemonic = phrase;
                    draft.passphrase = restore_pass.text().to_string();
                    draft.from_restore = true;
                    title.set_label("Set password");
                    stack.set_visible_child_name("password");
                }
                Err(err) => {
                    restore_error.set_label(err.as_label());
                    restore_error.set_visible(true);
                }
            }
        }
    ));

    password_back.connect_clicked(clone!(
        #[weak] stack,
        #[weak] title,
        #[strong] draft,
        move |_| {
            if draft.borrow().from_restore {
                title.set_label("Restore wallet");
                stack.set_visible_child_name("restore");
            } else {
                title.set_label("Confirm phrase");
                stack.set_visible_child_name("confirm");
            }
        }
    ));

    password_save.connect_clicked(clone!(
        #[weak] window,
        #[weak] password_input,
        #[weak] repeat_input,
        #[weak] password_error,
        #[strong] draft,
        #[strong] app_settings,
        move |_| {
            let password = password_input.text().to_string();
            let repeat = repeat_input.text().to_string();
            if let Err(err) = onboarding::validate_password(&password, &repeat) {
                password_error.set_label(err.as_label());
                password_error.set_visible(true);
                return;
            }
            let mnemonic = draft.borrow().mnemonic.clone();
            let passphrase = draft.borrow().passphrase.clone();
            if mnemonic.is_empty() {
                password_error.set_label("Recovery phrase is missing. Go back and try again.");
                password_error.set_visible(true);
                return;
            }
            password_error.set_visible(false);
            let saved = app_settings.lock().unwrap().finish_onboarding(
                &mnemonic,
                &passphrase,
                &password,
            );
            match saved {
                Ok(()) => {
                    password_input.set_text("");
                    repeat_input.set_text("");
                    *draft.borrow_mut() = Draft::default();
                    let settings = app_settings.lock().unwrap().clone();
                    stack::stack_view(&window, settings);
                }
                Err(_) => {
                    password_error.set_label("Could not save the wallet. Try again.");
                    password_error.set_visible(true);
                }
            }
        }
    ));

    import_back.connect_clicked(clone!(
        #[weak] stack,
        #[weak] title,
        move |_| {
            title.set_label("Set up wallet");
            stack.set_visible_child_name("choice");
        }
    ));
    import_save.connect_clicked(clone!(
        #[weak] window,
        #[weak] path_input,
        #[weak] import_error,
        #[strong] app_settings,
        move |_| {
            let path_text = path_input.text().to_string();
            if Path::new(&path_text).exists() && path_text.contains(".dic") {
                let dest = app_settings.lock().unwrap().config_path.clone();
                if dest.exists() {
                    let mut old = dest.clone();
                    old.pop();
                    old.push(format!("Config-Old-{}.dic", chrono::Local::now()));
                    let _ = fs::rename(&dest, old);
                }
                match fs::copy(&path_text, &dest) {
                    Ok(_) => tracing::info!("imported wallet store"),
                    Err(_) => {
                        crate::configuration::logging::error("failed to import wallet store");
                        import_error.set_visible(true);
                        return;
                    }
                }
                path_input.set_text("");
                login::login_view(window.clone(), app_settings.lock().unwrap().clone());
            } else {
                import_error.set_visible(true);
                path_input.set_text("");
            }
        }
    ));
}

fn start_create(
    word_count: WordCount,
    draft: &Rc<RefCell<Draft>>,
    seed_grid: &gtk::Grid,
    stack: &gtk::Stack,
    title: &gtk::Label,
    choice_error: &gtk::Label,
) {
    match onboarding::generate_create_phrase(word_count) {
        Ok(phrase) => {
            choice_error.set_visible(false);
            let mut draft = draft.borrow_mut();
            draft.mnemonic = phrase.clone();
            draft.passphrase.clear();
            draft.from_restore = false;
            fill_seed_grid(seed_grid, &phrase);
            title.set_label("Recovery phrase");
            stack.set_visible_child_name("show_seed");
        }
        Err(_) => {
            crate::configuration::logging::error("failed to generate recovery phrase");
            choice_error.set_visible(true);
        }
    }
}

fn fill_seed_grid(grid: &gtk::Grid, phrase: &str) {
    while let Some(child) = grid.first_child() {
        grid.remove(&child);
    }
    for (index, word) in phrase.split_whitespace().enumerate() {
        let label = gtk::Label::builder()
            .label(&format!("{}. {}", index + 1, word))
            .xalign(0.0)
            .hexpand(true)
            .selectable(true)
            .css_classes(["seed-word"])
            .build();
        grid.attach(&label, (index % 2) as i32, (index / 2) as i32, 1, 1);
    }
}

fn textview_content(view: &gtk::TextView) -> String {
    let buffer = view.buffer();
    buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), false)
        .to_string()
}

fn page_box() -> gtk::Box {
    gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_start(16)
        .margin_end(16)
        .margin_top(12)
        .margin_bottom(16)
        .spacing(10)
        .build()
}

fn setup_button(label: &str) -> Button {
    let button = Button::builder().label(label).hexpand(true).build();
    button.add_css_class("standard_button");
    button
}

fn wrapped_label(text: &str, classes: &[&str]) -> gtk::Label {
    let label = gtk::Label::builder()
        .label(text)
        .wrap(true)
        .wrap_mode(pango::WrapMode::WordChar)
        .justify(gtk::Justification::Center)
        .build();
    for class in classes {
        label.add_css_class(class);
    }
    label
}

fn error_label(text: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .wrap(true)
        .wrap_mode(pango::WrapMode::WordChar)
        .visible(false)
        .css_classes(["label-error"])
        .build()
}

fn seed_text_view() -> gtk::TextView {
    gtk::TextView::builder()
        .wrap_mode(gtk::WrapMode::WordChar)
        .accepts_tab(false)
        .left_margin(8)
        .right_margin(8)
        .top_margin(8)
        .bottom_margin(8)
        .build()
}

fn choice_page() -> (gtk::Box, Button, Button, Button, Button, gtk::Label) {
    let page = page_box();
    let logo_path = match ApplicationSettings::find_images_path() {
        Ok(mut path) => {
            path.push("Logo.png");
            path
        }
        Err(_) => PathBuf::new(),
    };
    let logo = gtk::Image::from_file(logo_path);
    logo.set_pixel_size(96);
    page.append(&logo);
    page.append(&wrapped_label(
        "Create a new wallet or restore from a recovery phrase.",
        &["label-standard"],
    ));

    let create_12 = setup_button("Create 12-word wallet");
    create_12.add_css_class("suggested-action");
    let create_24 = setup_button("Create 24-word wallet");
    let restore_btn = setup_button("Restore from recovery phrase");
    let import_btn = setup_button("Import wallet file");
    let choice_error = error_label("Could not generate a recovery phrase.");

    page.append(&create_12);
    page.append(&create_24);
    page.append(&restore_btn);
    page.append(&import_btn);
    page.append(&choice_error);
    (
        page,
        create_12,
        create_24,
        restore_btn,
        import_btn,
        choice_error,
    )
}

fn show_seed_page() -> (gtk::Box, gtk::Grid, Button, Button) {
    let page = page_box();
    page.append(&wrapped_label(
        "Write these words down in order and keep them offline. They will not be shown again after you continue.",
        &["seed-warning", "label-standard"],
    ));
    let seed_grid = gtk::Grid::builder()
        .column_spacing(12)
        .row_spacing(8)
        .column_homogeneous(true)
        .hexpand(true)
        .css_classes(["seed-grid"])
        .build();
    page.append(&seed_grid);
    let seed_continue = setup_button("I have written it down");
    seed_continue.add_css_class("suggested-action");
    let seed_back = setup_button("Back");
    page.append(&seed_continue);
    page.append(&seed_back);
    (page, seed_grid, seed_continue, seed_back)
}

fn confirm_page() -> (gtk::Box, gtk::TextView, Button, Button, gtk::Label) {
    let page = page_box();
    page.append(&wrapped_label(
        "Enter the recovery phrase to confirm you have it written down.",
        &["label-standard"],
    ));
    let confirm_view = seed_text_view();
    let scroll = gtk::ScrolledWindow::builder()
        .min_content_height(140)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&confirm_view)
        .build();
    page.append(&scroll);
    let confirm_error = error_label("Recovery phrase does not match.");
    let confirm_continue = setup_button("Confirm phrase");
    confirm_continue.add_css_class("suggested-action");
    let confirm_back = setup_button("Back");
    page.append(&confirm_error);
    page.append(&confirm_continue);
    page.append(&confirm_back);
    (page, confirm_view, confirm_continue, confirm_back, confirm_error)
}

fn restore_page() -> (gtk::Box, gtk::TextView, gtk::Entry, Button, Button, gtk::Label) {
    let page = page_box();
    page.append(&wrapped_label(
        "Enter your 12 or 24-word recovery phrase. Optional passphrase is the extra BIP39 word, not your app password.",
        &["label-standard"],
    ));
    let restore_view = seed_text_view();
    let scroll = gtk::ScrolledWindow::builder()
        .min_content_height(140)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&restore_view)
        .build();
    page.append(&scroll);
    let restore_pass = gtk::Entry::builder()
        .placeholder_text("Optional passphrase")
        .visibility(false)
        .hexpand(true)
        .build();
    page.append(&restore_pass);
    let restore_error = error_label("That recovery phrase is not valid.");
    let restore_continue = setup_button("Continue");
    restore_continue.add_css_class("suggested-action");
    let restore_back = setup_button("Back");
    page.append(&restore_error);
    page.append(&restore_continue);
    page.append(&restore_back);
    (
        page,
        restore_view,
        restore_pass,
        restore_continue,
        restore_back,
        restore_error,
    )
}

fn password_page() -> (gtk::Box, gtk::Entry, gtk::Entry, Button, Button, gtk::Label) {
    let page = page_box();
    page.append(&wrapped_label(
        "Choose a password to encrypt this wallet on this device. You will need it each time you unlock the app.",
        &["label-standard"],
    ));
    let password_input = gtk::Entry::builder()
        .placeholder_text("Password")
        .visibility(false)
        .hexpand(true)
        .build();
    let repeat_input = gtk::Entry::builder()
        .placeholder_text("Repeat password")
        .visibility(false)
        .hexpand(true)
        .build();
    let password_error = error_label("Passwords do not match.");
    let password_save = setup_button("Save wallet");
    password_save.add_css_class("suggested-action");
    let password_back = setup_button("Back");
    page.append(&password_input);
    page.append(&repeat_input);
    page.append(&password_error);
    page.append(&password_save);
    page.append(&password_back);
    (
        page,
        password_input,
        repeat_input,
        password_save,
        password_back,
        password_error,
    )
}

fn import_page() -> (gtk::Box, gtk::Entry, Button, Button, gtk::Label) {
    let page = page_box();
    page.append(&wrapped_label(
        "Import an existing Block Wallet file (.dic).",
        &["label-standard"],
    ));
    let path_input = gtk::Entry::builder()
        .placeholder_text("Path to .dic file")
        .hexpand(true)
        .build();
    let import_error = error_label("File not found or incorrect type.");
    let import_save = setup_button("Import");
    import_save.add_css_class("suggested-action");
    let import_back = setup_button("Back");
    page.append(&path_input);
    page.append(&import_error);
    page.append(&import_save);
    page.append(&import_back);
    (page, path_input, import_save, import_back, import_error)
}
