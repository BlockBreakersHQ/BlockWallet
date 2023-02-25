use gtk::prelude::*;
use gtk::{Orientation, Button};

use crate::currencies::tokens::Token;
use crate::ApplicationSettings;
use crate::views::{transactions};

pub fn currency_view(token: Token, app_settings: ApplicationSettings) -> gtk::Box {
    let currency_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(12)
        .margin_bottom(12)
        .build();

    let currency_label = gtk::Label::new(Some(&token.name));

    let scrollable_container = gtk::ScrolledWindow::builder()
        .child(&currency_label)
        .build();
    
    currency_box.append(&scrollable_container);
    return currency_box.clone();
}

pub fn transaction_button(ticker: String, app_settings: ApplicationSettings) -> gtk::Box {
    let button_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(12)
        .margin_bottom(12)
        .build();
    
    let transaction_button = Button::builder()
        .label(&format!("Send {}", ticker))
        .margin_top(6)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    transaction_button.add_css_class("standard_button");

    button_box.append(&transaction_button);

    transaction_button.connect_clicked(move |_| {
        transactions::transaction_view(app_settings.clone());
    });

    return button_box;
}