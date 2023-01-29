use std::time::Duration;
use std::thread;
use gtk::Orientation;
use crate::ApplicationSettings;

pub fn asset_view(app_settings: ApplicationSettings) -> gtk::Box {
    thread::spawn(move || {
        loop {
            let arc_balance = match &app_settings.eth_wallets[0].balance {
                Some(b) => b,
                None    => panic!("An error occurred in assets")
            };

            println!("app_settings_update.eth_wallets[0].balance = {}", *arc_balance.lock().unwrap());
            thread::sleep(Duration::from_secs(10));
        };
    });

    return gtk::Box::new(Orientation::Vertical, 15);
}
