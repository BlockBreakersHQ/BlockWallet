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

    let window_clone = window.clone();
    app_settings.update_balances();

    let header_bar = header_bar::header_bar_view(window.clone(), app_settings.clone());
    let stack = adw::ViewStack::new();
    let mut app_settings_clone = app_settings.clone();

    let home = home::home_view(window_clone.clone(), currency_pairs.clone());
    let home_label: Option<&str> = Some("Home");
    stack.add_titled(&home, home_label, "Home");

    let (wallet_box, app_settings) = wallets::wallet_view(app_settings);
    let wallet_label: Option<&str> = Some("Wallets");
    stack.add_titled(&wallet_box, wallet_label, "Wallets");

    let asset_label: Option<&str> = Some("Assets");
    stack.add_titled(&assets::asset_view(app_settings), asset_label, "Assets");

    let trade_label = gtk::Label::new(Some("Coming soon!"));
    stack.add_titled(&trade_label, Option::<&str>::None, "Trade");

    let stack_bar = adw::ViewSwitcherBar::new();
    stack_bar.set_widget_name("stack_bar");
    stack_bar.set_stack(Some(&stack));
    stack_bar.set_reveal(true);

    container.append(&stack_bar);
    container.prepend(&stack);
    container.prepend(&header_bar);    
    
    let stack_clone = stack.clone();
    let home_clone = home.clone();

    stack.connect_visible_child_notify(move |_| {
        if &stack_clone.visible_child_name().unwrap() == "Home" {
            home_clone.last_child().unwrap().set_visible(false);

            //home_box.first_child = scrollable_container, scrollable_container.first_child = GtkViewport, GtkViewport.first_child = scrollable_box
            home_clone.first_child().unwrap().first_child().unwrap().first_child().unwrap().set_visible(true);
        }
    });

    let _ = app_settings_clone.write_config();
    window.show();
}