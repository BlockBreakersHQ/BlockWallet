use gtk::prelude::*;
use gtk::{Orientation, Button};

use crate::ApplicationSettings;

pub fn transaction_view(app_settings: ApplicationSettings) -> (gtk::Box, ApplicationSettings) {
    let transaction_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();

    let send_address = gtk::Entry::builder()
        .placeholder_text("Send Address")
        .margin_top(12)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .build();

    let receive_address = gtk::Entry::builder()
        .placeholder_text("Recieve Address")
        .margin_top(12)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .build();

    let amount = gtk::Entry::builder()
        .placeholder_text("Amount")
        .margin_top(12)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .build();
    
    let advanced_button = Button::builder()
        .label("Show Advanced")
        .margin_top(6)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let gas = gtk::Entry::builder()
        .placeholder_text("Gas (Optional)")
        .margin_top(12)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .visible(false)
        .build();

    let gas_price = gtk::Entry::builder()
        .placeholder_text("Gas Price (Optional)")
        .margin_top(12)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .visible(false)
        .build();

    let data = gtk::Entry::builder()
        .placeholder_text("Data (Optional)")
        .margin_top(12)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .visible(false)
        .build();

    let submit_button = Button::builder()
        .label("Submit")
        .margin_top(6)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    
    submit_button.add_css_class("transaction_button");

    transaction_box.append(&send_address);
    transaction_box.append(&receive_address);
    transaction_box.append(&amount);
    transaction_box.append(&advanced_button);
    transaction_box.append(&gas);
    transaction_box.append(&gas_price);
    transaction_box.append(&data);
    transaction_box.append(&submit_button);

    advanced_button.connect_clicked(move |advanced_button| {
        if advanced_button.label() == Some("Hide Advanced".into()) {
            advanced_button.set_label("Show Advanced");
            gas.set_visible(false);
            gas_price.set_visible(false);
            data.set_visible(false);
        } else {
            advanced_button.set_label("Hide Advanced");
            gas.set_visible(true);
            gas_price.set_visible(true);
            data.set_visible(true);
        }
    });

    submit_button.connect_clicked(move |_| {
        println!("Submit button clicked");
    });

    return (transaction_box, app_settings);
}