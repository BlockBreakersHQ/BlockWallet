use gtk::prelude::*;
use gtk::CssProvider;
use gtk::gdk::Display;
use adw::{Application, ApplicationWindow};
use std::path::PathBuf;

use block_wallet::views::{login, new_wallet_generation};
use block_wallet::configuration::initialization;
use block_wallet::configuration::application_settings::ApplicationSettings;

const APP_ID: &str = "io.github.BlockBreakersHQ.BlockWallet";

/// The GTK application ID.
///
/// Inside Flatpak this must match the Flatpak app ID, or the Wayland surface is tagged
/// with the wrong app and Phosh shows the wrong icon and title in the switcher. The devel
/// manifest deliberately uses a `.Devel` suffix so it can be installed next to a release
/// build, so the ID cannot simply be hardcoded. `FLATPAK_ID` is set by the runtime and is
/// authoritative when present; outside Flatpak it is absent and the constant is used.
fn app_id() -> String {
    std::env::var("FLATPAK_ID").unwrap_or_else(|_| APP_ID.to_string())
}

fn main() {
    block_wallet::configuration::logging::init();

    let app = Application::builder().application_id(app_id()).build();
    app.connect_startup(|_| load_css());
    app.connect_startup(|_| load_images());
    app.connect_activate(build_ui);
    app.run();
}

fn load_images() {
    let icons = gtk::IconTheme::for_display(&Display::default().expect("Could not connect to a display."));
    if let Ok(path) = ApplicationSettings::find_images_path() {
        icons.add_search_path(path.join("Icons"));
    }
    if let Ok(path) = block_wallet::configuration::paths::icon_cache_path() {
        icons.add_search_path(path);
    }
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_data(include_str!("style.css"));

    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

pub fn build_ui(app: &Application) {
    let tokens = initialization::load_tokens();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("BlockWallet")
        .default_width(360)
        .default_height(720)
        .build();

    let cpath = match ApplicationSettings::find_config_path() {
        Ok(path) => path,
        Err(_) => PathBuf::new()
    };

    let window_clone = window.clone();
    let app_settings = ApplicationSettings::new(tokens);

    if cpath.exists() {
        login::login_view(window_clone, app_settings);
    } else {
        new_wallet_generation::generate_wallet_view(window_clone, app_settings);
    }
}
