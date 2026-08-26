//! Light/dark appearance.
//!
//! The app is built on libadwaita's named colours, so it follows the system theme for free and
//! tracks changes live. That is the right default, but it is not reachable on every setup: a
//! Flatpak reads the host's preference through the XDG desktop portal, and where no portal is
//! running the app simply never learns what the user chose. An in-app override means dark mode
//! is always available regardless.

use adw::prelude::*;

/// What the user picked in Settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appearance {
    /// Follow the system, which is libadwaita's own default behaviour.
    System = 0,
    Light = 1,
    Dark = 2,
}

impl Appearance {
    /// Map the stored string to a choice, defaulting to following the system.
    ///
    /// Unknown values fall back rather than erroring: this is a display preference, and a
    /// store written by a newer build should not stop an older one from opening.
    pub fn from_stored(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }

    pub fn from_index(index: u32) -> Self {
        match index {
            1 => Self::Light,
            2 => Self::Dark,
            _ => Self::System,
        }
    }

    pub fn as_stored(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    fn scheme(self) -> adw::ColorScheme {
        match self {
            // `Default` hands control back to the system rather than pinning either way.
            Self::System => adw::ColorScheme::Default,
            Self::Light => adw::ColorScheme::ForceLight,
            Self::Dark => adw::ColorScheme::ForceDark,
        }
    }

    /// Push the choice into libadwaita. Safe to call before any window exists.
    pub fn apply(self) {
        adw::StyleManager::default().set_color_scheme(self.scheme());
    }
}

/// Read the saved preference.
///
/// Kept in a small plaintext file under the config directory rather than inside the encrypted
/// store, for one reason: the store cannot be opened until the user has unlocked it, and the
/// unlock screen is the first thing they see. A theme setting that only took effect after
/// login would leave the lock screen in the wrong colours every launch. Appearance is not
/// secret, so there is nothing to protect here.
pub fn load() -> Appearance {
    let Ok(path) = crate::configuration::paths::appearance_path() else {
        return Appearance::System;
    };
    match std::fs::read_to_string(path) {
        Ok(value) => Appearance::from_stored(&value),
        // No file yet, which is the normal state on a fresh install.
        Err(_) => Appearance::System,
    }
}

/// Persist the preference. Best-effort: failing to save a theme must never block anything.
pub fn save(choice: Appearance) {
    if let Ok(path) = crate::configuration::paths::appearance_path() {
        let _ = std::fs::write(path, choice.as_stored());
    }
}

/// Apply whatever was saved. Called once at startup, before any window is shown.
pub fn apply_saved() {
    load().apply();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_values_round_trip() {
        for choice in [Appearance::System, Appearance::Light, Appearance::Dark] {
            assert_eq!(Appearance::from_stored(choice.as_stored()), choice);
        }
    }

    #[test]
    fn the_dropdown_index_matches_the_enum_order() {
        // The Settings combo is built as ["Follow system", "Light", "Dark"], and the row's
        // selected index is cast straight from this enum, so the two must not drift apart.
        assert_eq!(Appearance::System as u32, 0);
        assert_eq!(Appearance::Light as u32, 1);
        assert_eq!(Appearance::Dark as u32, 2);
        for index in 0..3u32 {
            assert_eq!(Appearance::from_index(index) as u32, index);
        }
    }

    #[test]
    fn anything_unrecognised_follows_the_system() {
        assert_eq!(Appearance::from_stored(""), Appearance::System);
        assert_eq!(Appearance::from_stored("sepia"), Appearance::System);
        assert_eq!(Appearance::from_stored("  DARK  "), Appearance::Dark);
        // A value written by some future build must not stop this one from opening the store.
        assert_eq!(Appearance::from_stored("high-contrast"), Appearance::System);
        assert_eq!(Appearance::from_index(99), Appearance::System);
    }
}
