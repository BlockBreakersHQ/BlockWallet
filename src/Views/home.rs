use gtk::prelude::*;
use gtk::{Orientation, Image, Align};
use glib::{clone, Continue, MainContext, PRIORITY_DEFAULT};
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

use crate::ApplicationSettings;
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
    let currency_box = gtk::Box::new(Orientation::Horizontal, 5);
    let icon_box     = gtk::Box::new(Orientation::Vertical, 0);
    let name_box     = gtk::Box::new(Orientation::Vertical, 0);
    let price_box    = gtk::Box::new(Orientation::Vertical, 0);

    let icon_path = match ApplicationSettings::find_images_path(){
        Ok(mut lp) => {
            lp.push("Icons");
            lp.push(element.0.0.image());
            lp
        },
        Err(_) => PathBuf::new()
    };

    let icon = Image::from_file(icon_path);
    icon.set_pixel_size(50);
    icon.set_margin_start(12);
    icon.set_margin_bottom(8);

    let currency_name  = gtk::Label::builder()
        .label(&element.0.0.name())
        .margin_top(5)
        .margin_start(5)
        .halign(Align::Start)
        .css_name("label-currency_name")
        .build();

    let currency_ticker  = gtk::Label::builder()
        .label(&element.0.0.ticker())
        .margin_top(5)
        .margin_start(5)
        .halign(Align::Start)
        .css_name("label-currency_ticker")
        .build();

    let currency_price_label  = gtk::Label::builder()
        .label(&*element.1.lock().unwrap())
        .margin_top(5)
        .margin_end(5)
        .halign(Align::End)
        .hexpand(true)
        .css_name("label-currency_price")
        .build();

    icon_box.append(&icon);
    name_box.append(&currency_name);
    name_box.append(&currency_ticker);
    price_box.append(&currency_price_label);
    currency_box.append(&icon_box);
    currency_box.append(&name_box);
    currency_box.append(&price_box);

    let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);

    thread::spawn(move || {
        loop {
            let out_string = element.1.lock().unwrap().clone();
            match sender.send(out_string) {
                Ok(_) => {},
                Err(_) => {}
            };
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