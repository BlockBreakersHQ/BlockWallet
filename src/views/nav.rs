use adw::prelude::*;
use gtk::{Button, Orientation};

/// In-tab list/detail/send navigation without walking the widget tree.
#[derive(Clone)]
pub struct Nav {
    pub stack: gtk::Stack,
    pub back: Button,
    /// Strip holding the back button. Owned here, and shown or hidden directly by
    /// `push`/`show_list`, so a pushed page always gets a visible way back and a list page
    /// never reserves an empty strip.
    bar: gtk::Box,
}

impl Nav {
    pub fn new(list: &impl IsA<gtk::Widget>) -> Self {
        let stack = gtk::Stack::builder()
            .vexpand(true)
            .transition_type(gtk::StackTransitionType::SlideLeftRight)
            .build();
        stack.add_named(list, Some("list"));
        stack.set_visible_child_name("list");

        // Icon plus "Back": on a phone an unlabelled arrow floating over content is easy
        // to miss, and this is the only way out of a detail or send page.
        let content = adw::ButtonContent::builder()
            .icon_name("go-previous-symbolic")
            .label("Back")
            .build();
        let back = Button::builder().child(&content).build();
        back.set_tooltip_text(Some("Back"));
        back.add_css_class("flat");
        back.set_valign(gtk::Align::Center);

        let bar = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(6)
            .margin_start(6)
            .margin_end(6)
            .margin_top(4)
            .visible(false)
            .build();
        bar.append(&back);

        let stack_back = stack.clone();
        let bar_back = bar.clone();
        back.connect_clicked(move |_| {
            let current = stack_back
                .visible_child_name()
                .unwrap_or_else(|| "list".into());
            if current == "send" && stack_back.child_by_name("detail").is_some() {
                stack_back.set_visible_child_name("detail");
            } else {
                stack_back.set_visible_child_name("list");
                bar_back.set_visible(false);
            }
        });

        Self { stack, back, bar }
    }

    pub fn wrap(self) -> gtk::Box {
        let outer = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .build();
        outer.append(&self.bar);
        outer.append(&self.stack);
        outer
    }

    pub fn push(&self, name: &str, child: &impl IsA<gtk::Widget>) {
        if let Some(existing) = self.stack.child_by_name(name) {
            self.stack.remove(&existing);
        }
        self.stack.add_named(child, Some(name));
        self.stack.set_visible_child_name(name);
        self.bar.set_visible(true);
    }

    pub fn show_list(&self) {
        self.stack.set_visible_child_name("list");
        self.bar.set_visible(false);
    }
}

pub fn parse_leading_amount(text: &str) -> f64 {
    text.split_whitespace()
        .find_map(|part| part.replace(',', "").parse::<f64>().ok())
        .unwrap_or(0.0)
}

pub fn label_is_offline(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("offline") || lower.contains("unreachable")
}

pub fn label_is_pending_sync(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("uninitialized") || lower.contains("syncing")
}

pub fn format_btc_units(display: &str, units: &str) -> String {
    if units.eq_ignore_ascii_case("sats") {
        let btc = parse_leading_amount(display);
        let sats = (btc * 100_000_000.0).round() as i64;
        let rest = display
            .find(' ')
            .map(|i| &display[i..])
            .unwrap_or("")
            .replace("BTC", "sats");
        return format!("{sats}{rest}");
    }
    display.to_string()
}

/// Inline status strip. Delegates to the shared notice styling so there is exactly one
/// banner look in the app.
pub fn banner(text: &str) -> gtk::Label {
    crate::views::ui::notice(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_leading_amount_reads_display_strings() {
        assert_eq!(parse_leading_amount("0.001 BTC"), 0.001);
        assert_eq!(parse_leading_amount("1.5 ETH (offline)"), 1.5);
        assert_eq!(parse_leading_amount("offline"), 0.0);
        assert_eq!(parse_leading_amount("Uninitialized"), 0.0);
    }

    #[test]
    fn format_btc_units_can_show_sats() {
        let out = format_btc_units("0.00010000 BTC", "sats");
        assert!(out.contains("10000"));
        assert!(out.contains("sats"));
        assert_eq!(format_btc_units("0.5 BTC", "btc"), "0.5 BTC");
    }

    #[test]
    fn offline_and_sync_labels() {
        assert!(label_is_offline("0.0 BTC (offline)"));
        assert!(label_is_pending_sync("Uninitialized"));
        assert!(!label_is_offline("0.1 ETH"));
    }
}
