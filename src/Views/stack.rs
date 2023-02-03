use gtk::prelude::*;
use adw::prelude::*;
use adw::{ApplicationWindow};

use crate::views::{assets, home, wallets, header_bar};
use crate::configuration::application_settings::*;
use crate::currencies::currency_pairs::CurrencyPairs;

pub fn stack_view(window: &ApplicationWindow, app_settings: ApplicationSettings) {
    let currency_pairs = CurrencyPairs::new(app_settings.clone());
    currency_pairs.update_token_balances();

    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    window.set_content(Some(&container));

    app_settings.update_balances();

    let header_bar = header_bar::header_bar_view(window.clone(), app_settings.clone());
    let stack = adw::ViewStack::new();
    let mut app_settings_clone = app_settings.clone();

    let home_label: Option<&str> = Some("Home");
    stack.add_titled(&home::home_view(currency_pairs), home_label, "Home");

    let (wallet_box, app_settings) = wallets::wallet_view(app_settings);
    let wallet_label: Option<&str> = Some("Wallets");
    stack.add_titled(&wallet_box, wallet_label, "Wallets");

    let asset_label: Option<&str> = Some("Assets");
    stack.add_titled(&assets::asset_view(app_settings), asset_label, "Assets");

    let trade_label = gtk::Label::new(Some("Coming soon!"));
    stack.add_titled(&trade_label, Option::<&str>::None, "Trade");

    let stack_bar = adw::ViewSwitcherBar::new();
    stack_bar.set_stack(Some(&stack));
    stack_bar.set_reveal(true);

    container.append(&header_bar);
    container.append(&stack_bar);
    container.append(&stack);

    let _ = app_settings_clone.write_config();
    window.show();
}