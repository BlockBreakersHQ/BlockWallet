use adw::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk::{Button, Image, Orientation};
use pango::WrapMode;
use std::sync::{Arc, Mutex};

use crate::configuration::application_settings::*;
use crate::configuration::clipboard;
use crate::configuration::seed;
use crate::configuration::wallet_display;
use crate::currencies::btc;
use crate::currencies::btc::BitcoinWallet;
use crate::currencies::eth;
use crate::currencies::eth::EthereumWallet;
use crate::currencies::ltc;
use crate::currencies::ltc::LitecoinWallet;
use crate::currencies::sol;
use crate::currencies::sol::SolanaWallet;

pub fn wallet_view(app_settings: Arc<Mutex<ApplicationSettings>>) -> (gtk::Box, Arc<Mutex<ApplicationSettings>>) {
    let btc_data_displayed = Arc::new(Mutex::new(false));
    let eth_data_displayed = Arc::new(Mutex::new(false));
    let sol_data_displayed = Arc::new(Mutex::new(false));
    let ltc_data_displayed = Arc::new(Mutex::new(false));

    let btc_wallets = app_settings.lock().unwrap().btc_wallets.clone();
    let eth_wallets = app_settings.lock().unwrap().eth_wallets.clone();
    let sol_wallets = app_settings.lock().unwrap().sol_wallets.clone();
    let ltc_wallets = app_settings.lock().unwrap().ltc_wallets.clone();

    let scrollable_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(6)
        .margin_bottom(6)
        .vexpand(true)
        .build();
    scrollable_box.set_widget_name("wallet_scrollable_box");

    let scrollable_container = gtk::ScrolledWindow::builder()
        .child(&scrollable_box)
        .name("wallet_scrollable_container")
        .build();

    let btc_button = setup_button("Bitcoin");
    let eth_button = setup_button("Ethereum");
    let sol_button = setup_button("Solana");
    let ltc_button = setup_button("Litecoin");
    let add_wallet_button = setup_button("Add account");

    let wallet_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(6)
        .margin_bottom(6)
        .build();

    scrollable_box.append(&btc_button);
    scrollable_box.append(&eth_button);
    scrollable_box.append(&sol_button);
    scrollable_box.append(&ltc_button);
    scrollable_box.append(&add_wallet_button);

    let btc_currency_details = populate_btc_currency_details(&btc_wallets, app_settings.clone());
    let eth_currency_details = populate_eth_currency_details(&eth_wallets, app_settings.clone());
    let sol_currency_details = populate_sol_currency_details(&sol_wallets, app_settings.clone());
    let ltc_currency_details = populate_ltc_currency_details(&ltc_wallets, app_settings.clone());
    scrollable_box.insert_child_after(&btc_currency_details, Some(&btc_button));
    scrollable_box.insert_child_after(&eth_currency_details, Some(&eth_button));
    scrollable_box.insert_child_after(&sol_currency_details, Some(&sol_button));
    scrollable_box.insert_child_after(&ltc_currency_details, Some(&ltc_button));
    let btc_currency_details_clone = btc_currency_details.clone();
    let eth_currency_details_clone = eth_currency_details.clone();
    let btc_currency_details_clone2 = btc_currency_details.clone();
    let eth_currency_details_clone2 = eth_currency_details.clone();
    let btc_currency_details_clone3 = btc_currency_details.clone();
    let eth_currency_details_clone3 = eth_currency_details.clone();
    let sol_currency_details_clone = sol_currency_details.clone();
    let sol_currency_details_clone2 = sol_currency_details.clone();
    let sol_currency_details_clone3 = sol_currency_details.clone();
    let ltc_currency_details_clone = ltc_currency_details.clone();
    let ltc_currency_details_clone2 = ltc_currency_details.clone();
    let ltc_currency_details_clone3 = ltc_currency_details.clone();

    let new_wallet_box = new_wallet_box(
        app_settings.clone(),
        Arc::new(Mutex::new(btc_currency_details.clone())),
        Arc::new(Mutex::new(eth_currency_details.clone())),
        Arc::new(Mutex::new(sol_currency_details.clone())),
        Arc::new(Mutex::new(ltc_currency_details.clone())),
        Arc::new(Mutex::new(scrollable_container.clone())),
    );
    wallet_box.append(&scrollable_container);
    wallet_box.append(&new_wallet_box);

    btc_button.connect_clicked(move |_| {
        let mut shown = btc_data_displayed.lock().unwrap();
        *shown = !*shown;
        btc_currency_details.set_visible(*shown);
        if *shown {
            eth_currency_details_clone.set_visible(false);
            sol_currency_details_clone.set_visible(false);
            ltc_currency_details_clone.set_visible(false);
        }
    });

    eth_button.connect_clicked(move |_| {
        let mut shown = eth_data_displayed.lock().unwrap();
        *shown = !*shown;
        eth_currency_details.set_visible(*shown);
        if *shown {
            btc_currency_details_clone.set_visible(false);
            sol_currency_details_clone2.set_visible(false);
            ltc_currency_details_clone2.set_visible(false);
        }
    });

    sol_button.connect_clicked(move |_| {
        let mut shown = sol_data_displayed.lock().unwrap();
        *shown = !*shown;
        sol_currency_details.set_visible(*shown);
        if *shown {
            btc_currency_details_clone2.set_visible(false);
            eth_currency_details_clone2.set_visible(false);
            ltc_currency_details_clone3.set_visible(false);
        }
    });

    ltc_button.connect_clicked(move |_| {
        let mut shown = ltc_data_displayed.lock().unwrap();
        *shown = !*shown;
        ltc_currency_details.set_visible(*shown);
        if *shown {
            btc_currency_details_clone3.set_visible(false);
            eth_currency_details_clone3.set_visible(false);
            sol_currency_details_clone3.set_visible(false);
        }
    });

    add_wallet_button.connect_clicked(move |_| {
        new_wallet_box.set_visible(true);
        scrollable_container.set_visible(false);
    });

    (wallet_box, app_settings)
}

fn populate_btc_currency_details(
    btc_wallets: &[BitcoinWallet],
    app_settings: Arc<Mutex<ApplicationSettings>>,
) -> gtk::Box {
    let widgets = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .visible(false)
        .build();
    for wallet in btc_wallets {
        append_account_card(
            &widgets,
            wallet.wallet_name.as_deref(),
            wallet.address.as_deref(),
            "BTC",
            btc_qr_box(wallet),
            app_settings.clone(),
        );
    }
    widgets
}

fn populate_eth_currency_details(
    eth_wallets: &[EthereumWallet],
    app_settings: Arc<Mutex<ApplicationSettings>>,
) -> gtk::Box {
    let widgets = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .visible(false)
        .build();
    for wallet in eth_wallets {
        append_account_card(
            &widgets,
            wallet.wallet_name.as_deref(),
            wallet.address.as_deref(),
            "ETH",
            eth_qr_box(wallet),
            app_settings.clone(),
        );
    }
    widgets
}

fn populate_sol_currency_details(
    sol_wallets: &[SolanaWallet],
    app_settings: Arc<Mutex<ApplicationSettings>>,
) -> gtk::Box {
    let widgets = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .visible(false)
        .build();
    for wallet in sol_wallets {
        append_account_card(
            &widgets,
            wallet.wallet_name.as_deref(),
            wallet.address.as_deref(),
            "SOL",
            sol_qr_box(wallet),
            app_settings.clone(),
        );
    }
    widgets
}

fn populate_ltc_currency_details(
    ltc_wallets: &[LitecoinWallet],
    app_settings: Arc<Mutex<ApplicationSettings>>,
) -> gtk::Box {
    let widgets = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .visible(false)
        .build();
    for wallet in ltc_wallets {
        append_account_card(
            &widgets,
            wallet.wallet_name.as_deref(),
            wallet.address.as_deref(),
            "LTC",
            ltc_qr_box(wallet),
            app_settings.clone(),
        );
    }
    widgets
}

fn append_account_card(
    parent: &gtk::Box,
    name: Option<&str>,
    address: Option<&str>,
    icon: &str,
    qr_box: gtk::Box,
    app_settings: Arc<Mutex<ApplicationSettings>>,
) {
    let lines = wallet_display::default_visible_lines(name, address);
    let title = lines[0].clone();
    let address_text = address.unwrap_or("").to_string();
    let subtitle = if address_text.is_empty() {
        String::new()
    } else {
        wallet_display::truncate_address(&address_text)
    };

    let address_label = gtk::Label::builder()
        .label(&lines[1])
        .halign(gtk::Align::Start)
        .hexpand(true)
        .wrap(true)
        .wrap_mode(WrapMode::Char)
        .selectable(true)
        .css_classes(["btc-wallet-details"])
        .margin_start(12)
        .margin_end(12)
        .build();

    let copy_address = setup_button("Copy address");
    let address_for_copy = address_text.clone();
    copy_address.connect_clicked(move |_| {
        if !address_for_copy.is_empty() {
            clipboard::copy_text(&address_for_copy);
        }
    });

    let qr_button = setup_button("Show QR code");
    qr_box.set_halign(gtk::Align::Center);
    let qr_box_toggle = qr_box.clone();
    qr_button.connect_clicked(move |button| {
        let show = !qr_box_toggle.is_visible();
        qr_box_toggle.set_visible(show);
        button.set_label(if show { "Hide QR code" } else { "Show QR code" });
    });

    let expander = adw::ExpanderRow::builder()
        .title(&title)
        .subtitle(&subtitle)
        .margin_start(12)
        .margin_end(12)
        .height_request(44)
        .css_classes(["wallet-expander"])
        .icon_name(icon)
        .build();
    expander.add_row(&address_label);
    expander.add_row(&copy_address);
    expander.add_row(&qr_box);
    expander.add_row(&qr_button);
    expander.add_row(&reveal_box(address_text, app_settings));

    parent.append(&expander);
}

fn reveal_box(address: String, app_settings: Arc<Mutex<ApplicationSettings>>) -> gtk::Box {
    let box_ = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(8)
        .build();

    let reveal_button = setup_button("Reveal backup details");
    let password = gtk::Entry::builder()
        .placeholder_text("Password")
        .visibility(false)
        .hexpand(true)
        .visible(false)
        .build();
    let unlock_details = setup_button("Show details");
    unlock_details.set_visible(false);
    let error = gtk::Label::builder()
        .label("Password incorrect.")
        .wrap(true)
        .visible(false)
        .css_classes(["label-error"])
        .build();
    let phrase_label = gtk::Label::builder()
        .halign(gtk::Align::Start)
        .wrap(true)
        .wrap_mode(WrapMode::WordChar)
        .selectable(true)
        .visible(false)
        .css_classes(["seed-word"])
        .build();
    let key_label = gtk::Label::builder()
        .halign(gtk::Align::Start)
        .wrap(true)
        .wrap_mode(WrapMode::Char)
        .selectable(true)
        .visible(false)
        .css_classes(["seed-word"])
        .build();
    let copy_phrase = setup_button("Copy recovery phrase");
    copy_phrase.set_visible(false);
    let hide_button = setup_button("Hide details");
    hide_button.set_visible(false);

    box_.append(&reveal_button);
    box_.append(&password);
    box_.append(&unlock_details);
    box_.append(&error);
    box_.append(&phrase_label);
    box_.append(&key_label);
    box_.append(&copy_phrase);
    box_.append(&hide_button);

    reveal_button.connect_clicked(clone!(
        #[weak] password,
        #[weak] unlock_details,
        #[weak] error,
        move |button| {
            error.set_visible(false);
            password.set_visible(true);
            unlock_details.set_visible(true);
            button.set_visible(false);
        }
    ));

    unlock_details.connect_clicked(clone!(
        #[weak] password,
        #[weak] error,
        #[weak] phrase_label,
        #[weak] key_label,
        #[weak] copy_phrase,
        #[weak] hide_button,
        #[weak] unlock_details,
        #[strong] app_settings,
        #[strong] address,
        move |_| {
            let entered = password.text().to_string();
            password.set_text("");
            if !app_settings.lock().unwrap().verify_password(&entered) {
                error.set_visible(true);
                return;
            }
            error.set_visible(false);
            let (mnemonic, private_key) = app_settings.lock().unwrap().secrets_for_address(&address);
            if let Some(phrase) = mnemonic.filter(|value| !value.is_empty()) {
                phrase_label.set_label(&format!("Recovery phrase: {phrase}"));
                phrase_label.set_visible(true);
                copy_phrase.set_visible(true);
            }
            if let Some(key) = private_key.filter(|value| !value.is_empty()) {
                key_label.set_label(&format!("Private key: {key}"));
                key_label.set_visible(true);
            }
            hide_button.set_visible(true);
            unlock_details.set_visible(false);
            password.set_visible(false);
        }
    ));

    copy_phrase.connect_clicked(clone!(
        #[weak] phrase_label,
        move |_| {
            let text = phrase_label.label().to_string();
            let phrase = text
                .strip_prefix("Recovery phrase: ")
                .unwrap_or(text.as_str())
                .to_string();
            if !phrase.is_empty() {
                clipboard::copy_text_timed(&phrase);
            }
        }
    ));

    hide_button.connect_clicked(clone!(
        #[weak] phrase_label,
        #[weak] key_label,
        #[weak] copy_phrase,
        #[weak] hide_button,
        #[weak] reveal_button,
        #[weak] password,
        #[weak] error,
        move |_| {
            phrase_label.set_label("");
            key_label.set_label("");
            phrase_label.set_visible(false);
            key_label.set_visible(false);
            copy_phrase.set_visible(false);
            hide_button.set_visible(false);
            password.set_text("");
            password.set_visible(false);
            error.set_visible(false);
            reveal_button.set_visible(true);
        }
    ));

    box_
}

fn add_btc_wallet(
    btc_box: &gtk::Box,
    btcw: &BitcoinWallet,
    app_settings: Arc<Mutex<ApplicationSettings>>,
) {
    append_account_card(
        btc_box,
        btcw.wallet_name.as_deref(),
        btcw.address.as_deref(),
        "BTC",
        btc_qr_box(btcw),
        app_settings,
    );
}

fn add_eth_wallet(
    eth_box: &gtk::Box,
    ethw: &EthereumWallet,
    app_settings: Arc<Mutex<ApplicationSettings>>,
) {
    append_account_card(
        eth_box,
        ethw.wallet_name.as_deref(),
        ethw.address.as_deref(),
        "ETH",
        eth_qr_box(ethw),
        app_settings,
    );
}

fn add_sol_wallet(
    sol_box: &gtk::Box,
    solw: &SolanaWallet,
    app_settings: Arc<Mutex<ApplicationSettings>>,
) {
    append_account_card(
        sol_box,
        solw.wallet_name.as_deref(),
        solw.address.as_deref(),
        "SOL",
        sol_qr_box(solw),
        app_settings,
    );
}

fn add_ltc_wallet(
    ltc_box: &gtk::Box,
    ltcw: &LitecoinWallet,
    app_settings: Arc<Mutex<ApplicationSettings>>,
) {
    append_account_card(
        ltc_box,
        ltcw.wallet_name.as_deref(),
        ltcw.address.as_deref(),
        "LTC",
        ltc_qr_box(ltcw),
        app_settings,
    );
}

pub fn btc_qr_box(btcw: &BitcoinWallet) -> gtk::Box {
    qr_box_from_texture(btcw.generate_qr_address().ok())
}

pub fn eth_qr_box(ethw: &EthereumWallet) -> gtk::Box {
    qr_box_from_texture(ethw.generate_qr_address().ok())
}

pub fn sol_qr_box(solw: &SolanaWallet) -> gtk::Box {
    qr_box_from_texture(solw.generate_qr_address().ok())
}

pub fn ltc_qr_box(ltcw: &LitecoinWallet) -> gtk::Box {
    qr_box_from_texture(ltcw.generate_qr_address().ok())
}

fn qr_box_from_texture(texture: Option<gtk::gdk::Texture>) -> gtk::Box {
    let qr_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .visible(false)
        .halign(gtk::Align::Center)
        .build();
    if let Some(qr_code) = texture {
        let qr_image = Image::from_paintable(Some(&qr_code));
        qr_image.set_pixel_size(160);
        qr_box.append(&qr_image);
    }
    qr_box
}

fn new_wallet_box(
    app_settings: Arc<Mutex<ApplicationSettings>>,
    btc_box: Arc<Mutex<gtk::Box>>,
    eth_box: Arc<Mutex<gtk::Box>>,
    sol_box: Arc<Mutex<gtk::Box>>,
    ltc_box: Arc<Mutex<gtk::Box>>,
    scrollable_container: Arc<Mutex<gtk::ScrolledWindow>>,
) -> gtk::Box {
    let new_wallet_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .visible(false)
        .spacing(8)
        .margin_start(12)
        .margin_end(12)
        .build();

    let intro = gtk::Label::builder()
        .label("Add another account from this recovery phrase, or import a key. The phrase is never shown here.")
        .wrap(true)
        .wrap_mode(WrapMode::WordChar)
        .css_classes(["label-standard"])
        .build();

    let tokens = ["Bitcoin", "Ethereum", "Solana", "Litecoin"];
    let token_selector = gtk::DropDown::from_strings(&tokens);
    let wallet_name = gtk::Entry::builder()
        .placeholder_text("Account name")
        .hexpand(true)
        .build();
    let create_wallet_button = setup_button("Add from recovery phrase");
    create_wallet_button.add_css_class("suggested-action");
    let create_error = gtk::Label::builder()
        .label("Could not add that account.")
        .wrap(true)
        .visible(false)
        .css_classes(["label-error"])
        .build();

    let import_token_selector = gtk::DropDown::from_strings(&tokens);
    let import_wallet_name = gtk::Entry::builder()
        .placeholder_text("Imported account name")
        .hexpand(true)
        .build();
    let import_secret = gtk::Entry::builder()
        .placeholder_text("WIF, private key, or recovery phrase")
        .visibility(false)
        .hexpand(true)
        .build();
    let import_wallet_button = setup_button("Import");
    let import_error = gtk::Label::builder()
        .label("Could not import that key.")
        .wrap(true)
        .visible(false)
        .css_classes(["label-error"])
        .build();
    let back_button = setup_button("Back");

    new_wallet_box.append(&intro);
    new_wallet_box.append(&token_selector);
    new_wallet_box.append(&wallet_name);
    new_wallet_box.append(&create_wallet_button);
    new_wallet_box.append(&create_error);
    new_wallet_box.append(&import_token_selector);
    new_wallet_box.append(&import_wallet_name);
    new_wallet_box.append(&import_secret);
    new_wallet_box.append(&import_wallet_button);
    new_wallet_box.append(&import_error);
    new_wallet_box.append(&back_button);

    let form = new_wallet_box.clone();
    back_button.connect_clicked(clone!(
        #[strong] scrollable_container,
        move |_| {
            form.set_visible(false);
            scrollable_container.lock().unwrap().set_visible(true);
        }
    ));

    create_wallet_button.connect_clicked(clone!(
        #[strong] app_settings,
        #[strong] eth_box,
        #[strong] sol_box,
        #[strong] scrollable_container,
        #[weak] token_selector,
        #[weak] wallet_name,
        #[weak] create_error,
        #[weak] new_wallet_box,
        move |_| {
            create_error.set_visible(false);
            let name = wallet_name.text().to_string();
            let mut settings = app_settings.lock().unwrap();
            let Some(phrase) = settings.mnemonic.clone() else {
                create_error.set_label("Unlock the wallet before adding an account.");
                create_error.set_visible(true);
                return;
            };
            let passphrase = settings.seed_passphrase.clone().unwrap_or_default();
            if token_selector.selected() == 0 {
                create_error.set_label("This seed already has a Bitcoin account. Import a WIF to add a different key.");
                create_error.set_visible(true);
                return;
            }
            if token_selector.selected() == 3 {
                create_error.set_label("This seed already has a Litecoin account. Import a WIF to add a different key.");
                create_error.set_visible(true);
                return;
            }
            if token_selector.selected() == 2 {
                // Extra Solana accounts increment the hardened account index (m/44'/501'/n'/0'),
                // matching Phantom/Solflare's convention, since ed25519 SLIP-10 requires every
                // component to be hardened (no non-hardened /0/n suffix like the ETH path uses).
                let path = format!("m/44'/501'/{}'/0'", settings.sol_wallets.len());
                let mut solw = match SolanaWallet::from_mnemonic(&phrase, &path, &passphrase) {
                    Ok(wallet) => wallet,
                    Err(_) => {
                        create_error.set_visible(true);
                        return;
                    }
                };
                if !name.is_empty() {
                    solw.set_wallet_name(name);
                }
                add_sol_wallet(&sol_box.lock().unwrap(), &solw, app_settings.clone());
                settings.sol_wallets.push(solw);
                let _ = settings.write_config();
                new_wallet_box.set_visible(false);
                scrollable_container.lock().unwrap().set_visible(true);
                return;
            }
            let path = format!("m/44'/60'/0'/0/{}", settings.eth_wallets.len());
            let mut ethw = match seed::ethereum_from_seed(&phrase, &path, &passphrase, &name) {
                Ok(wallet) => wallet,
                Err(_) => {
                    create_error.set_visible(true);
                    return;
                }
            };
            if !name.is_empty() {
                ethw.set_wallet_name(name);
            }
            add_eth_wallet(&eth_box.lock().unwrap(), &ethw, app_settings.clone());
            settings.eth_wallets.push(ethw);
            let _ = settings.write_config();
            new_wallet_box.set_visible(false);
            scrollable_container.lock().unwrap().set_visible(true);
        }
    ));

    import_wallet_button.connect_clicked(clone!(
        #[strong] app_settings,
        #[strong] btc_box,
        #[strong] eth_box,
        #[strong] sol_box,
        #[strong] ltc_box,
        #[strong] scrollable_container,
        #[weak] import_token_selector,
        #[weak] import_wallet_name,
        #[weak] import_secret,
        #[weak] import_error,
        #[weak] new_wallet_box,
        move |_| {
            import_error.set_visible(false);
            let secret = import_secret.text().to_string();
            import_secret.set_text("");
            let name = import_wallet_name.text().to_string();
            let mut settings = app_settings.lock().unwrap();
            if import_token_selector.selected() == 0 {
                let mut btcw = match btc::generate_from_private_key(&secret) {
                    Some(wallet) => wallet,
                    None => {
                        import_error.set_visible(true);
                        return;
                    }
                };
                if !name.is_empty() {
                    btcw.set_wallet_name(name);
                }
                add_btc_wallet(&btc_box.lock().unwrap(), &btcw, app_settings.clone());
                settings.btc_wallets.push(btcw);
            } else if import_token_selector.selected() == 2 {
                let mut solw = if secret.contains(' ') {
                    match sol::generate_from_mnemonic(&secret, seed::SOL_PATH) {
                        Some(wallet) => wallet,
                        None => {
                            import_error.set_visible(true);
                            return;
                        }
                    }
                } else {
                    match sol::generate_from_private_key(&secret) {
                        Some(wallet) => wallet,
                        None => {
                            import_error.set_visible(true);
                            return;
                        }
                    }
                };
                if !name.is_empty() {
                    solw.set_wallet_name(name);
                }
                add_sol_wallet(&sol_box.lock().unwrap(), &solw, app_settings.clone());
                settings.sol_wallets.push(solw);
            } else if import_token_selector.selected() == 3 {
                let mut ltcw = match ltc::generate_from_private_key(&secret) {
                    Some(wallet) => wallet,
                    None => {
                        import_error.set_visible(true);
                        return;
                    }
                };
                if !name.is_empty() {
                    ltcw.set_wallet_name(name);
                }
                add_ltc_wallet(&ltc_box.lock().unwrap(), &ltcw, app_settings.clone());
                settings.ltc_wallets.push(ltcw);
            } else if secret.contains(' ') {
                let mut ethw = match eth::generate_from_mnemonic(&secret, seed::ETH_PATH) {
                    Some(wallet) => wallet,
                    None => {
                        import_error.set_visible(true);
                        return;
                    }
                };
                if !name.is_empty() {
                    ethw.set_wallet_name(name);
                }
                add_eth_wallet(&eth_box.lock().unwrap(), &ethw, app_settings.clone());
                settings.eth_wallets.push(ethw);
            } else {
                let mut ethw = match eth::generate_from_private_key(&secret) {
                    Some(wallet) => wallet,
                    None => {
                        import_error.set_visible(true);
                        return;
                    }
                };
                if !name.is_empty() {
                    ethw.set_wallet_name(name);
                }
                add_eth_wallet(&eth_box.lock().unwrap(), &ethw, app_settings.clone());
                settings.eth_wallets.push(ethw);
            }
            let _ = settings.write_config();
            new_wallet_box.set_visible(false);
            scrollable_container.lock().unwrap().set_visible(true);
        }
    ));

    new_wallet_box
}

fn setup_button(label: &str) -> Button {
    let button = Button::builder()
        .label(label)
        .hexpand(true)
        .margin_start(12)
        .margin_end(12)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    button.add_css_class("standard_button");
    button
}
