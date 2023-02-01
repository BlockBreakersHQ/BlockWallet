use gtk::prelude::*;
use gtk::Orientation;
use glib::{clone, Continue, MainContext, PRIORITY_DEFAULT};
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};

use crate::currencies::currency_pairs::*;

pub fn home_view(currency_pairs: CurrencyPairs) -> gtk::Box {
    let home_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(12)
        .margin_bottom(12)
        .build();
    
    for element in currency_pairs.pairs {
        let currency_box = generate_currency_box(element);
        home_box.append(&currency_box);
    }
    
    return home_box.clone();
}

pub fn generate_currency_box(element: ((Token, Token), Arc<Mutex<String>>)) -> gtk::Box {
    let currency_box = gtk::Box::new(Orientation::Horizontal, 185);
    let currency_label  = gtk::Label::builder()
        .label(&element.0.0.name())
        .margin_top(12)
        .margin_start(50)
        .build();

    let currency_price_label  = gtk::Label::builder()
        .label(&*element.1.lock().unwrap())
        .margin_top(12)
        .margin_end(50)
        .build();

    currency_box.append(&currency_label);
    currency_box.append(&currency_price_label);

    let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);

    thread::spawn(move || {
        loop {
            let out_string = element.1.lock().unwrap().clone();
            sender.send(out_string).expect("Could not send through channel");
            thread::sleep(Duration::from_secs(1));
        }
    });

    receiver.attach(
        None,
        clone!(@weak currency_box => @default-return Continue(false),
            move |price_text| {
                if price_text != "Uninitialized" {
                    currency_price_label.set_label(&price_text);
                }
                Continue(true)
            }
        ),
    );
    
    return currency_box;
}