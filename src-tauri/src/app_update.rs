use std::{
    sync::{Mutex, MutexGuard},
    thread,
    time::Duration,
};

use serde::{Serialize, Serializer};
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_updater::{Update, UpdaterExt};
use thiserror::Error;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Default)]
pub struct AppUpdateState {
    pending_update: Mutex<Option<Update>>,
}

impl AppUpdateState {
    fn store_update(&self, update: Update) -> Result<(), AppUpdateError> {
        let mut pending_update = self.lock_pending_update()?;
        pending_update.replace(update);

        Ok(())
    }

    fn take_update(&self) -> Result<Option<Update>, AppUpdateError> {
        let mut pending_update = self.lock_pending_update()?;

        Ok(pending_update.take())
    }

    fn clear_update(&self) -> Result<(), AppUpdateError> {
        let mut pending_update = self.lock_pending_update()?;
        pending_update.take();

        Ok(())
    }

    fn lock_pending_update(&self) -> Result<MutexGuard<'_, Option<Update>>, AppUpdateError> {
        self.pending_update.lock().map_err(|_| {
            AppUpdateError::new(
                "update_state_unavailable",
                "Update state is unavailable. Restart Floci UI and try again.",
            )
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AppUpdateSnapshot {
    pub available: bool,
    pub current_version: String,
    pub version: Option<String>,
    pub date: Option<String>,
    pub body: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AppUpdateInstallResult {
    pub installed: bool,
    pub message: String,
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("{message}")]
pub struct AppUpdateError {
    pub code: &'static str,
    pub message: String,
}

impl AppUpdateInstallResult {
    fn installed() -> Self {
        Self {
            installed: true,
            message: "Update installed. Floci UI is restarting.".to_owned(),
        }
    }
}

impl AppUpdateError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn no_pending_update() -> Self {
        Self::new(
            "no_pending_update",
            "No pending update is available. Check for updates before installing.",
        )
    }
}

impl Serialize for AppUpdateError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.message)
    }
}

impl From<tauri_plugin_updater::Error> for AppUpdateError {
    fn from(error: tauri_plugin_updater::Error) -> Self {
        Self::new("updater_failed", error.to_string())
    }
}

#[tauri::command]
pub async fn app_update_check<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppUpdateState>,
) -> Result<AppUpdateSnapshot, AppUpdateError> {
    let update = app.updater()?.check().await?;

    if let Some(update) = update {
        let snapshot = available_snapshot(
            update.current_version.clone(),
            update.version.clone(),
            update.date,
            update.body.clone(),
        )?;
        state.store_update(update)?;

        Ok(snapshot)
    } else {
        state.clear_update()?;
        Ok(unavailable_snapshot())
    }
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn app_update_install<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppUpdateState>,
) -> Result<AppUpdateInstallResult, AppUpdateError> {
    let update = state
        .take_update()?
        .ok_or_else(AppUpdateError::no_pending_update)?;

    update.download_and_install(|_, _| {}, || {}).await?;
    let result = AppUpdateInstallResult::installed();
    schedule_restart(app);

    Ok(result)
}

fn schedule_restart<R: Runtime>(app: AppHandle<R>) {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(250));
        app.request_restart();
    });
}

fn unavailable_snapshot() -> AppUpdateSnapshot {
    AppUpdateSnapshot {
        available: false,
        current_version: env!("CARGO_PKG_VERSION").to_owned(),
        version: None,
        date: None,
        body: None,
    }
}

fn available_snapshot(
    current_version: String,
    version: String,
    date: Option<OffsetDateTime>,
    body: Option<String>,
) -> Result<AppUpdateSnapshot, AppUpdateError> {
    Ok(AppUpdateSnapshot {
        available: true,
        current_version,
        version: Some(version),
        date: format_update_date(date)?,
        body,
    })
}

fn format_update_date(date: Option<OffsetDateTime>) -> Result<Option<String>, AppUpdateError> {
    date.map(|date| {
        date.format(&Rfc3339).map_err(|_| {
            AppUpdateError::new(
                "invalid_update_date",
                "Update metadata included a date that could not be formatted.",
            )
        })
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, to_value};
    use time::OffsetDateTime;

    use super::{AppUpdateError, AppUpdateInstallResult, available_snapshot, unavailable_snapshot};

    #[test]
    fn unavailable_snapshot_uses_current_package_version() {
        let snapshot = unavailable_snapshot();

        assert!(!snapshot.available);
        assert_eq!(snapshot.current_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(snapshot.version, None);
        assert_eq!(snapshot.date, None);
        assert_eq!(snapshot.body, None);
    }

    #[test]
    fn available_snapshot_formats_release_metadata() {
        let date =
            OffsetDateTime::from_unix_timestamp(1_800_000_000).expect("timestamp should parse");
        let snapshot = available_snapshot(
            "0.1.0".to_owned(),
            "0.2.0".to_owned(),
            Some(date),
            Some("Release notes".to_owned()),
        )
        .expect("snapshot should format");

        assert!(snapshot.available);
        assert_eq!(snapshot.current_version, "0.1.0");
        assert_eq!(snapshot.version.as_deref(), Some("0.2.0"));
        assert_eq!(snapshot.date.as_deref(), Some("2027-01-15T08:00:00Z"));
        assert_eq!(snapshot.body.as_deref(), Some("Release notes"));
    }

    #[test]
    fn no_pending_update_error_serializes_message() {
        let value: Value =
            to_value(AppUpdateError::no_pending_update()).expect("error should serialize");

        assert_eq!(
            value.as_str(),
            Some("No pending update is available. Check for updates before installing.")
        );
    }

    #[test]
    fn install_result_has_stable_success_payload() {
        let result = AppUpdateInstallResult::installed();

        assert!(result.installed);
        assert_eq!(result.message, "Update installed. Floci UI is restarting.");
    }
}
