#![allow(clippy::must_use_candidate)]

#[cfg(target_arch = "wasm32")]
const THEME_STORAGE_KEY: &str = "floci-ui-theme";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThemeMode {
    Light,
    Dark,
}

impl ThemeMode {
    pub fn as_attribute_value(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThemePreference {
    System,
    Light,
    Dark,
}

impl ThemePreference {
    pub fn from_storage_value(value: &str) -> Self {
        match value {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }

    pub fn as_storage_value(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn resolved_mode(self) -> ThemeMode {
        match self {
            Self::System => detect_system_theme(),
            Self::Light => ThemeMode::Light,
            Self::Dark => ThemeMode::Dark,
        }
    }
}

pub fn initialize_theme() -> ThemeMode {
    apply_theme_preference(read_theme_preference())
}

#[cfg(target_arch = "wasm32")]
pub fn read_theme_preference() -> ThemePreference {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(THEME_STORAGE_KEY).ok().flatten())
        .map_or(ThemePreference::System, |value| {
            ThemePreference::from_storage_value(&value)
        })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_theme_preference() -> ThemePreference {
    ThemePreference::System
}

#[cfg(target_arch = "wasm32")]
pub fn write_theme_preference(preference: ThemePreference) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item(THEME_STORAGE_KEY, preference.as_storage_value());
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_theme_preference(_preference: ThemePreference) {}

#[cfg(target_arch = "wasm32")]
pub fn apply_theme_preference(preference: ThemePreference) -> ThemeMode {
    let mode = preference.resolved_mode();

    if let Some(root) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.document_element())
    {
        let _ = root.set_attribute("data-theme", mode.as_attribute_value());
        let _ = root.set_attribute("data-theme-preference", preference.as_storage_value());
    }

    mode
}

#[cfg(not(target_arch = "wasm32"))]
pub fn apply_theme_preference(preference: ThemePreference) -> ThemeMode {
    preference.resolved_mode()
}

#[cfg(target_arch = "wasm32")]
fn detect_system_theme() -> ThemeMode {
    web_sys::window()
        .and_then(|window| {
            window
                .match_media("(prefers-color-scheme: dark)")
                .ok()
                .flatten()
        })
        .map_or(ThemeMode::Light, |query| {
            if query.matches() {
                ThemeMode::Dark
            } else {
                ThemeMode::Light
            }
        })
}

#[cfg(not(target_arch = "wasm32"))]
fn detect_system_theme() -> ThemeMode {
    ThemeMode::Light
}

#[cfg(test)]
mod tests {
    use super::{ThemeMode, ThemePreference};

    #[test]
    fn parses_theme_preference_storage_values() {
        assert_eq!(
            ThemePreference::from_storage_value("system"),
            ThemePreference::System
        );
        assert_eq!(
            ThemePreference::from_storage_value("light"),
            ThemePreference::Light
        );
        assert_eq!(
            ThemePreference::from_storage_value("dark"),
            ThemePreference::Dark
        );
        assert_eq!(
            ThemePreference::from_storage_value("unexpected"),
            ThemePreference::System
        );
    }

    #[test]
    fn serializes_theme_preference_storage_values() {
        assert_eq!(ThemePreference::System.as_storage_value(), "system");
        assert_eq!(ThemePreference::Light.as_storage_value(), "light");
        assert_eq!(ThemePreference::Dark.as_storage_value(), "dark");
    }

    #[test]
    fn exposes_theme_mode_attribute_values() {
        assert_eq!(ThemeMode::Light.as_attribute_value(), "light");
        assert_eq!(ThemeMode::Dark.as_attribute_value(), "dark");
    }
}
