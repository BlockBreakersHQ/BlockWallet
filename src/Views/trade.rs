use crate::ApplicationSettings;
use gtk::prelude::*;
use gtk::{Orientation, Label};

pub fn trade_view(app_settings: ApplicationSettings) -> (gtk::Box, ApplicationSettings) {
    let transaction_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();

    let trade_label  = gtk::Label::builder()
        .label("trade view")
        .margin_top(5)
        .margin_start(5)
        .build();
    
    transaction_box.append(&trade_label);
    return (transaction_box, app_settings)
}