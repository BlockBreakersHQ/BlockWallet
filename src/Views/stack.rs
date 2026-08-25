use adw::prelude::*;
use adw::ApplicationWindow;
use glib::{clone, ControlFlow};
use gtk::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::configuration::application_settings::*;
use crate::views::{activity, assets, header_bar, home, login, wallets};

pub fn stack_view(window: &ApplicationWindow, app_settings_orig: ApplicationSettings) {
    let app_settings = Arc::new(Mutex::new(app_settings_orig));
    app_settings.lock().unwrap().update_balances();

    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header_bar = header_bar::header_bar_view(window.clone(), app_settings.clone());
    let stack = adw::ViewStack::new();

    let (home_box, home_nav, app_settings) = home::home_view(app_settings);
    stack.add_titled(&home_box, Some("Home"), "Home").set_icon_name(Some("home"));

    let (wallet_box, app_settings) = wallets::wallet_view(app_settings);
    stack.add_titled(&wallet_box, Some("Wallets"), "Wallets").set_icon_name(Some("wallet"));

    let (asset_box, asset_nav, app_settings) = assets::asset_view(app_settings);
    stack.add_titled(&asset_box, Some("Assets"), "Assets").set_icon_name(Some("assets"));

    let activity_box = activity::activity_view(app_settings.clone());
    stack
        .add_titled(&activity_box, Some("Activity"), "Activity")
        .set_icon_name(Some("view-list-symbolic"));

    let stack_bar = adw::ViewSwitcherBar::new();
    stack_bar.set_widget_name("stack_bar");
    stack_bar.set_stack(Some(&stack));
    stack_bar.set_reveal(true);

    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(520);
    clamp.set_child(Some(&stack));

    container.append(&header_bar);
    container.append(&clamp);
    container.append(&stack_bar);

    window.set_content(Some(&container));

    stack.connect_visible_child_notify(clone!(
        #[strong] home_nav,
        #[strong] asset_nav,
        move |stack| {
            let name = stack.visible_child_name().unwrap_or_default();
            if name != "Home" {
                home_nav.show_list();
            }
            if name != "Assets" {
                asset_nav.show_list();
            }
        }
    ));

    let last_input = Arc::new(Mutex::new(Instant::now()));
    let motion = gtk::EventControllerMotion::new();
    motion.connect_motion(clone!(
        #[strong] last_input,
        move |_, _, _| {
            *last_input.lock().unwrap() = Instant::now();
        }
    ));
    window.add_controller(motion);

    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(clone!(
        #[strong] last_input,
        move |_, _, _, _| {
            *last_input.lock().unwrap() = Instant::now();
            glib::Propagation::Proceed
        }
    ));
    window.add_controller(keys);

    glib::timeout_add_seconds_local(
        15,
        clone!(
            #[weak] window,
            #[strong] app_settings,
            #[strong] last_input,
            #[upgrade_or]
            ControlFlow::Break,
            move || {
                let timeout = app_settings.lock().unwrap().lock_timeout_secs;
                if timeout > 0
                    && last_input.lock().unwrap().elapsed() > Duration::from_secs(timeout as u64)
                    && app_settings.lock().unwrap().is_unlocked()
                {
                    login::lock_and_show(window.clone(), app_settings.clone());
                }
                ControlFlow::Continue
            }
        ),
    );

    window.connect_close_request(clone!(
        #[strong] app_settings,
        move |_| {
            let _ = app_settings.lock().unwrap().write_config();
            glib::Propagation::Proceed
        }
    ));

    window.present();
}
