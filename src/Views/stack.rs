use gtk::prelude::*;
use gtk::Inhibit;
use adw::prelude::*;
use adw::{ApplicationWindow};
use std::sync::{Arc, Mutex};

use crate::views::{assets, home, wallets, header_bar, trade};
use crate::configuration::application_settings::*;
use crate::currencies::currency_pairs::CurrencyPairs;

pub fn stack_view(window: &ApplicationWindow, app_settings_orig: ApplicationSettings) {
    let app_settings = Arc::new(Mutex::new(app_settings_orig));
    let currency_pairs = CurrencyPairs::new(app_settings.lock().unwrap().clone());
    currency_pairs.update_token_balances();
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);

    window.set_content(Some(&container));
    app_settings.lock().unwrap().update_balances();

    let header_bar = header_bar::header_bar_view(window.clone(), app_settings.clone());
    let stack = adw::ViewStack::new();

    let (home_box, app_settings) = home::home_view(currency_pairs.clone(), app_settings);
    let home_label: Option<&str> = Some("Home");
    stack.add_titled_with_icon(&home_box, home_label, "Home", "home");

    let (wallet_box, app_settings) = wallets::wallet_view(app_settings);
    let wallet_label: Option<&str> = Some("Wallets");
    stack.add_titled_with_icon(&wallet_box, wallet_label, "Wallets", "wallet");

    let (asset_box, app_settings) = assets::asset_view(app_settings);
    let asset_label: Option<&str> = Some("Assets");
    stack.add_titled_with_icon(&asset_box, asset_label, "Assets", "assets");

    let (trade_box, app_settings) = trade::trade_view(app_settings.clone());
    let trade_label: Option<&str> = Some("Trade");
    stack.add_titled_with_icon(&trade_box, trade_label, "Trade", "trade");

    let stack_bar = adw::ViewSwitcherBar::new();
    stack_bar.set_widget_name("stack_bar");
    stack_bar.set_stack(Some(&stack));
    stack_bar.set_reveal(true);

    container.append(&stack_bar);
    container.prepend(&stack);
    container.prepend(&header_bar);    
    
    let stack_clone = stack.clone();
    let home_clone = home_box.clone();
    let asset_clone = asset_box.clone();

    stack.connect_visible_child_notify(move |_| {
        if &stack_clone.visible_child_name().unwrap() == "Home" {
            home_clone.last_child().unwrap().set_visible(false);

            //home_box.first_child = scrollable_container, scrollable_container.first_child = GtkViewport, GtkViewport.first_child = scrollable_box
            home_clone.first_child().unwrap().set_visible(true);
        }
        else if &stack_clone.visible_child_name().unwrap() == "Assets" {
            // assets_scrollable_container -> GtkViewport -> assets_scrollable_box -> currencies_box
            let assets_scrollable_container = match asset_clone.first_child() {
                Some(child) => child,
                None => return,
            };
            let gtk_viewport = match assets_scrollable_container.first_child() {
                Some(child) => child,
                None => return,
            };
            let assets_scrollable_box = match gtk_viewport.first_child() {
                Some(child) => child,
                None => return,
            };
            let currencies_box = match assets_scrollable_box.first_child() {
                Some(child) => child,
                None => return,
            };
            let no_assets_label = match currencies_box.first_child() {
                Some(child) => child,
                None => return,
            };
            let currency_detail = match no_assets_label.next_sibling() {
                Some(child) => child,
                None => return,
            };
            currency_detail.set_visible(false);

            let next = match currency_detail.next_sibling() {
                Some(child) => child,
                None => currencies_box.last_child().unwrap()
            };

            if next.widget_name().contains("currency_detail") {
                next.set_visible(false);
            }

            currencies_box.last_child().unwrap().set_visible(true);
        }
    });

    window.connect_close_request(move |_| {
        let _ = app_settings.lock().unwrap().write_config();
        Inhibit(false)
    });

    window.show();
}