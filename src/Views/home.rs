use adw::prelude::*;
use gtk::{Orientation, Image, Align};
use glib::{clone, Continue, MainContext, PRIORITY_DEFAULT};
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};

use crate::currencies::currency_pairs::*;
use crate::currencies::tokens::Token;
use crate::views::currency::currency_view;
use crate::ApplicationSettings;

pub fn home_view(currency_pairs: CurrencyPairs, app_settings: Arc<Mutex<ApplicationSettings>>) -> (gtk::Box, Arc<Mutex<ApplicationSettings>>) {
    let home_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(12)
        .margin_bottom(12)
        .build();

    let scrollable_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .vexpand(true)
        .build();
    scrollable_box.set_widget_name("home_scrollable_box");
    
    let currency_detail_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .visible(false)
        .build();

    let scrollable_container = gtk::ScrolledWindow::builder()
        .child(&scrollable_box)
        .name("scrollable_container")
        .build();

    for element in currency_pairs.pairs {
        let currency_box = generate_currency_box(element.clone());
        let currency_detail_clone = currency_detail_box.clone();
        let scroll_clone = scrollable_container.clone();
        let gesture = gtk::GestureClick::new();
        let e = element.0.clone();
        let app_settings = app_settings.clone();
        let app_settings = app_settings.lock().unwrap().clone();
        gesture.connect_released(move |gesture, _, _, _| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            scroll_clone.set_visible(false);
            
            match currency_detail_clone.first_child() {
                Some(fc) => currency_detail_clone.remove(&fc),
                None => {}
            };

            currency_detail_clone.append(&currency_view(e.clone(), app_settings.clone()));
            currency_detail_clone.set_visible(true);
        });
        currency_box.add_controller(&gesture);
        scrollable_box.append(&currency_box);
    }
    
    home_box.append(&scrollable_container);
    home_box.append(&currency_detail_box);

    (home_box, app_settings)
}

pub fn generate_currency_box(element: (Token, Arc<Mutex<String>>)) -> gtk::Box {
    let currency_box = gtk::Box::new(Orientation::Horizontal, 5);
    let icon_box     = gtk::Box::new(Orientation::Vertical, 0);
    let name_box     = gtk::Box::new(Orientation::Vertical, 0);
    let price_box    = gtk::Box::new(Orientation::Vertical, 0);

    let icon = Image::from_file(element.0.logo);
    
    icon.set_pixel_size(50);
    icon.set_margin_start(12);
    icon.set_margin_bottom(8);

    let currency_name  = gtk::Label::builder()
        .label(&element.0.name)
        .margin_top(5)
        .margin_start(5)
        .halign(Align::Start)
        .css_name("label-currency_name")
        .build();

    let currency_ticker  = gtk::Label::builder()
        .label(&element.0.symbol)
        .margin_top(5)
        .margin_start(5)
        .halign(Align::Start)
        .css_name("label-currency_ticker")
        .build();

    let currency_price_label  = gtk::Label::builder()
        .label(&*element.1.lock().unwrap())
        .margin_top(5)
        .margin_end(12)
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