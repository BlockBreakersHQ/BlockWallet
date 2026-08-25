use glib::ControlFlow;
use gtk::gdk;
use gtk::gdk::prelude::DisplayExt;

pub const CLIPBOARD_CLEAR_SECS: u32 = 30;

pub fn copy_text_timed(text: &str) {
    let Some(display) = gdk::Display::default() else {
        return;
    };
    let clipboard = display.clipboard();
    clipboard.set_text(text);
    glib::timeout_add_seconds_local(CLIPBOARD_CLEAR_SECS, move || {
        clipboard.set_text("");
        ControlFlow::Break
    });
}

pub fn copy_text(text: &str) {
    copy_text_timed(text);
}

#[cfg(test)]
mod tests {
    #[test]
    fn secret_clipboard_clears_within_a_minute() {
        assert!(super::CLIPBOARD_CLEAR_SECS >= 10);
        assert!(super::CLIPBOARD_CLEAR_SECS <= 60);
    }
}
