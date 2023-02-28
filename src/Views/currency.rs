use gtk::prelude::*;
use gtk::{Orientation, Button};

use crate::currencies::tokens::Token;
use crate::ApplicationSettings;
use crate::views::{transactions};

pub fn currency_view(token: Token, app_settings: ApplicationSettings) -> gtk::Box {
    let currency_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        //.margin_top(12)
        //.margin_bottom(12)
        .build();

    let currency_label = gtk::Label::new(Some(&token.name));
    
    let transaction_detail_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        //.margin_top(12)
        //.margin_bottom(12)
        .visible(false)
        .build();
    
    let currency_detail_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        //.margin_top(12)
        //.margin_bottom(12)
        .build();

    let transaction_button = Button::builder()
        .label(&format!("Send {}", token.symbol))
        .margin_top(6)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    transaction_button.add_css_class("standard_button");
    
    let scrollable_container = gtk::ScrolledWindow::builder()
        .child(&currency_detail_box)
        .vexpand(true)
        .build();
        
    currency_box.append(&scrollable_container);
    currency_box.append(&transaction_detail_box);
    currency_detail_box.append(&currency_label);
    currency_detail_box.append(&transaction_button);

    transaction_button.connect_clicked(move |_| {
        transaction_detail_box.set_visible(true);
        scrollable_container.set_visible(false);
        transaction_detail_box.append(&transactions::transaction_view(app_settings.clone()).0);
    });
    
    return currency_box.clone();
}