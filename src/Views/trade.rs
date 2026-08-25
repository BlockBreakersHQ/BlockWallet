use gtk::prelude::*;
use pango::WrapMode;
use std::sync::{Arc, Mutex};

use crate::ApplicationSettings;

pub fn trade_view(
    app_settings: Arc<Mutex<ApplicationSettings>>,
) -> (gtk::Box, Arc<Mutex<ApplicationSettings>>) {
    let trade_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .margin_start(16)
        .margin_end(16)
        .margin_top(24)
        .margin_bottom(16)
        .spacing(10)
        .build();

    let title = gtk::Label::builder()
        .label("Swaps are not available yet")
        .css_classes(["title"])
        .wrap(true)
        .justify(gtk::Justification::Center)
        .build();
    let body = gtk::Label::builder()
        .label("This release sends and receives Bitcoin and Ethereum on layer 1. Token swaps come later.")
        .wrap(true)
        .wrap_mode(WrapMode::WordChar)
        .justify(gtk::Justification::Center)
        .css_classes(["label-standard"])
        .build();

    trade_box.append(&title);
    trade_box.append(&body);
    (trade_box, app_settings)
}
