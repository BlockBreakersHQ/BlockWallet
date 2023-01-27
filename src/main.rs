use gtk::prelude::*;
use gtk::{CssProvider, StyleContext};
use gtk::gdk::{Display};
use adw::{Application, ApplicationWindow};

mod views;
mod currencies;
mod configuration;

use crate::views::{login};
use crate::configuration::application_settings::*;

const APP_ID: &str = "org.BlockBreakers.Wallet";

//#[tokio::main]
fn main() {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| load_css());
    app.connect_activate(build_ui);
    app.run();
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_data(include_bytes!("style.css"));

    StyleContext::add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

pub fn build_ui(app: &Application) {    
   let window = ApplicationWindow::builder()
        .application(app)
        .title("BlockWallet")
        .default_width(360)
        .default_height(720)
        .build(); 

    let window_clone = window.clone();
    let app_settings = ApplicationSettings::new();
    login::login_view(window_clone, app_settings);
}