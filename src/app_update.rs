use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct AppUpdateSnapshot {
    pub available: bool,
    pub current_version: String,
    pub version: Option<String>,
    pub date: Option<String>,
    pub body: Option<String>,
}

impl AppUpdateSnapshot {
    pub fn target_version_label(&self) -> &str {
        self.version.as_deref().unwrap_or("new version")
    }

    pub fn release_notes_text(&self) -> String {
        match self
            .body
            .as_deref()
            .map(str::trim)
            .filter(|body| !body.is_empty())
        {
            Some(body) => body.to_owned(),
            None => "No release notes were provided for this update.".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct AppUpdateInstallResult {
    pub installed: bool,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum UpdateCheckState {
    #[default]
    Idle,
    Checking,
    Unavailable,
    Available(AppUpdateSnapshot),
    Failed(String),
    Installing,
    Installed(String),
}

impl UpdateCheckState {
    pub fn available_snapshot(&self) -> Option<&AppUpdateSnapshot> {
        match self {
            Self::Available(snapshot) => Some(snapshot),
            _ => None,
        }
    }

    pub fn can_install(&self) -> bool {
        matches!(self, Self::Available(_))
    }

    pub fn is_installing(&self) -> bool {
        matches!(self, Self::Installing)
    }
}

#[cfg(test)]
mod tests {
    use super::{AppUpdateSnapshot, UpdateCheckState};

    fn available_snapshot() -> AppUpdateSnapshot {
        AppUpdateSnapshot {
            available: true,
            current_version: "0.1.0".to_owned(),
            version: Some("v0.2.0".to_owned()),
            date: Some("2026-05-17T00:00:00Z".to_owned()),
            body: Some("  Fixes and polish.  ".to_owned()),
        }
    }

    #[test]
    fn release_notes_text_trims_notes() {
        let snapshot = available_snapshot();

        assert_eq!(snapshot.release_notes_text(), "Fixes and polish.");
    }

    #[test]
    fn release_notes_text_falls_back_when_empty() {
        let mut snapshot = available_snapshot();
        snapshot.body = Some("   ".to_owned());

        assert_eq!(
            snapshot.release_notes_text(),
            "No release notes were provided for this update."
        );
    }

    #[test]
    fn target_version_label_falls_back_without_version() {
        let mut snapshot = available_snapshot();
        snapshot.version = None;

        assert_eq!(snapshot.target_version_label(), "new version");
    }

    #[test]
    fn update_state_allows_install_only_when_available() {
        let state = UpdateCheckState::Available(available_snapshot());

        assert!(state.can_install());
        assert!(state.available_snapshot().is_some());
        assert!(!UpdateCheckState::Checking.can_install());
        assert!(!UpdateCheckState::Installing.can_install());
        assert!(UpdateCheckState::Installing.is_installing());
    }
}
