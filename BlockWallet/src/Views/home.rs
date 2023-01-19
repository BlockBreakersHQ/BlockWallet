use gtk::prelude::*;
use gtk::StackTransitionType::SlideLeftRight;
use gtk::{Button, Image};
use adw::prelude::*;
use adw::{ApplicationWindow, HeaderBar};

use crate::{assets, settings};
use crate::configuration::ApplicationSettings;

pub fn home_view(window: &ApplicationWindow, app_settings: ApplicationSettings) {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    window.set_content(Some(&container));

    let header_bar = HeaderBar::new();
    let settings_button = Button::new();
    
    let settings_icon = Image::from_file("cog.png");
    settings_icon.set_pixel_size(25);
    settings_button.set_child(Some(&settings_icon));

    header_bar.pack_start(&settings_button);
    
    let stack = gtk::Stack::new();

    let mut app_settings_clone = app_settings.clone();

    stack.set_transition_type(SlideLeftRight);
    stack.set_transition_duration(200);

    let home_label = gtk::Label::new(Some("Home"));
    stack.add_titled(&home_label, Option::<&str>::None, "Home");

    let opt: Option<&str> = Some("Assets");
    stack.add_titled(&assets::asset_view(app_settings), opt, "Assets");

    let trade_label = gtk::Label::new(Some("Trade"));
    stack.add_titled(&trade_label, Option::<&str>::None, "Trade");

    let stack_switcher = gtk::StackSwitcher::new();
    stack_switcher.set_stack(Some(&stack));

    container.append(&header_bar);
    container.append(&stack_switcher);
    container.append(&stack);

    let _ = app_settings_clone.write_config();
    window.show();
    let window_clone = window.clone();
    let app_clone = app_settings_clone.clone();

    settings_button.connect_clicked(move |_| {
        settings::settings_view(window_clone.clone(), app_clone.clone());
    });
}