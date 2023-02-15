use gtk::prelude::*;
use gtk::{Orientation};
use crate::currencies::tokens::Token;

pub fn currency_view(token: Token) -> gtk::Box {
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