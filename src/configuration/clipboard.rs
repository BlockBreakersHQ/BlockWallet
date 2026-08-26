use glib::ControlFlow;
use gtk::gdk;
use gtk::gdk::prelude::DisplayExt;
// `read_text_async` and `set_text` live on the GDK clipboard traits that the gtk prelude
// re-exports; the gdk prelude alone does not carry them.
use gtk::prelude::*;

/// How long a secret copied to the clipboard is allowed to live there.
pub const CLIPBOARD_CLEAR_SECS: u32 = 30;

/// Copy something that is not secret, such as a receive address.
///
/// Deliberately never auto-clears. This used to delegate to [`copy_text_timed`], which meant
/// copying an address armed a timer that wiped the clipboard 30 seconds later — and it wiped
/// whatever was on the clipboard *by then*, not what this put there. Copy an address here,
/// then copy a recipient address in another app, and the stale timer would erase it before it
/// could be pasted. That is what made pasting into the recipient field look broken.
pub fn copy_text(text: &str) {
    let Some(display) = gdk::Display::default() else {
        return;
    };
    display.clipboard().set_text(text);
}

/// Copy a secret, and clear it again shortly afterwards.
///
/// For recovery phrases and private keys, where leaving the value on a shared clipboard is a
/// real exposure. The clear is conditional: it only fires if the clipboard still holds exactly
/// what was put there, so anything the user has copied since is left alone.
pub fn copy_text_timed(text: &str) {
    let Some(display) = gdk::Display::default() else {
        return;
    };
    let clipboard = display.clipboard();
    clipboard.set_text(text);

    let expected = text.to_string();
    glib::timeout_add_seconds_local(CLIPBOARD_CLEAR_SECS, move || {
        let expected = expected.clone();
        // Read back before clearing. `read_text_async` is the only way to see the current
        // contents, and by the time it answers the user may have copied something else again,
        // which is why the comparison happens in the callback rather than before it.
        clipboard.read_text_async(gtk::gio::Cancellable::NONE, {
            let clipboard = clipboard.clone();
            move |result| {
                let still_ours = matches!(result, Ok(Some(ref current)) if current.as_str() == expected);
                if still_ours {
                    clipboard.set_text("");
                }
            }
        });
        ControlFlow::Break
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn secret_clipboard_clears_within_a_minute() {
        assert!(super::CLIPBOARD_CLEAR_SECS >= 10);
        assert!(super::CLIPBOARD_CLEAR_SECS <= 60);
    }
}
