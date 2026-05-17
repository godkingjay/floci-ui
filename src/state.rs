use leptos::*;
use wasm_bindgen_futures::spawn_local;

use crate::{
    app_update::{AppUpdateInstallResult, AppUpdateSnapshot, UpdateCheckState},
    commands::invoke_command,
    models::{DashboardSnapshot, HealthSnapshot},
    routes::AppRoute,
    theme::{
        ThemeMode, ThemePreference, apply_theme_preference, read_theme_preference,
        write_theme_preference,
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RemoteData<T> {
    Loading,
    Ready(T),
    Failed(String),
}

impl<T> RemoteData<T> {
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RefreshState {
    pub health_loading: bool,
    pub catalog_loading: bool,
}

impl RefreshState {
    pub fn is_loading(self) -> bool {
        self.health_loading || self.catalog_loading
    }
}

#[derive(Clone, Copy)]
pub struct AppStore {
    pub health: ReadSignal<RemoteData<HealthSnapshot>>,
    pub set_health: WriteSignal<RemoteData<HealthSnapshot>>,
    pub catalog: ReadSignal<RemoteData<DashboardSnapshot>>,
    pub set_catalog: WriteSignal<RemoteData<DashboardSnapshot>>,
    pub refresh: ReadSignal<RefreshState>,
    pub set_refresh: WriteSignal<RefreshState>,
    pub refresh_generation: ReadSignal<u64>,
    pub set_refresh_generation: WriteSignal<u64>,
    pub route: ReadSignal<AppRoute>,
    pub set_route: WriteSignal<AppRoute>,
    pub theme_preference: ReadSignal<ThemePreference>,
    pub set_theme_preference: WriteSignal<ThemePreference>,
    pub theme_mode: ReadSignal<ThemeMode>,
    pub set_theme_mode: WriteSignal<ThemeMode>,
    pub update: ReadSignal<UpdateCheckState>,
    pub set_update: WriteSignal<UpdateCheckState>,
    pub active_update: ReadSignal<Option<AppUpdateSnapshot>>,
    pub set_active_update: WriteSignal<Option<AppUpdateSnapshot>>,
    pub update_dialog_open: ReadSignal<bool>,
    pub set_update_dialog_open: WriteSignal<bool>,
}

impl AppStore {
    pub fn create() -> Self {
        let (health, set_health) = create_signal(RemoteData::Loading);
        let (catalog, set_catalog) = create_signal(RemoteData::Loading);
        let (refresh, set_refresh) = create_signal(RefreshState::default());
        let (refresh_generation, set_refresh_generation) = create_signal(0);
        let (route, set_route) = create_signal(AppRoute::current());
        let initial_preference = read_theme_preference();
        let (theme_preference, set_theme_preference) = create_signal(initial_preference);
        let (theme_mode, set_theme_mode) = create_signal(initial_preference.resolved_mode());
        let (update, set_update) = create_signal(UpdateCheckState::Idle);
        let (active_update, set_active_update) = create_signal(None);
        let (update_dialog_open, set_update_dialog_open) = create_signal(false);

        Self {
            health,
            set_health,
            catalog,
            set_catalog,
            refresh,
            set_refresh,
            refresh_generation,
            set_refresh_generation,
            route,
            set_route,
            theme_preference,
            set_theme_preference,
            theme_mode,
            set_theme_mode,
            update,
            set_update,
            active_update,
            set_active_update,
            update_dialog_open,
            set_update_dialog_open,
        }
    }

    pub fn refresh_dashboard(self) {
        let generation = self.refresh_generation.get_untracked().saturating_add(1);
        self.set_refresh_generation.set(generation);
        self.set_health.set(RemoteData::Loading);
        self.set_catalog.set(RemoteData::Loading);
        self.set_refresh.set(RefreshState {
            health_loading: true,
            catalog_loading: true,
        });

        spawn_local(async move {
            let (health, catalog) = futures::join!(
                invoke_command::<HealthSnapshot>("floci_health"),
                invoke_command::<DashboardSnapshot>("service_catalog"),
            );

            if !is_current_refresh_generation(self.refresh_generation.get_untracked(), generation) {
                return;
            }

            self.set_health.set(remote_result(health));
            self.set_catalog.set(remote_result(catalog));
            self.set_refresh.set(RefreshState::default());
        });
    }

    pub fn check_for_updates(self) {
        self.set_update.set(UpdateCheckState::Checking);

        spawn_local(async move {
            let update = invoke_command::<AppUpdateSnapshot>("app_update_check").await;

            let state = match update {
                Ok(snapshot) if snapshot.available => {
                    self.set_active_update.set(Some(snapshot.clone()));
                    UpdateCheckState::Available(snapshot)
                }
                Ok(_) => {
                    self.set_active_update.set(None);
                    UpdateCheckState::Unavailable
                }
                Err(message) => UpdateCheckState::Failed(message),
            };

            self.set_update.set(state);
        });
    }

    pub fn install_update(self) {
        self.set_update.set(UpdateCheckState::Installing);

        spawn_local(async move {
            let result = invoke_command::<AppUpdateInstallResult>("app_update_install").await;

            let state = match result {
                Ok(result) if result.installed => UpdateCheckState::Installed(result.message),
                Ok(result) => UpdateCheckState::Failed(result.message),
                Err(message) => UpdateCheckState::Failed(message),
            };

            self.set_update.set(state);
        });
    }

    pub fn toggle_theme(self) {
        let next_preference = match self.theme_mode.get_untracked() {
            ThemeMode::Light => ThemePreference::Dark,
            ThemeMode::Dark => ThemePreference::Light,
        };
        let next_mode = apply_theme_preference(next_preference);

        write_theme_preference(next_preference);
        self.set_theme_preference.set(next_preference);
        self.set_theme_mode.set(next_mode);
    }
}

fn remote_result<T>(result: Result<T, String>) -> RemoteData<T> {
    match result {
        Ok(value) => RemoteData::Ready(value),
        Err(message) => RemoteData::Failed(message),
    }
}

fn is_current_refresh_generation(current: u64, candidate: u64) -> bool {
    current == candidate
}

#[cfg(test)]
mod tests {
    use super::is_current_refresh_generation;

    #[test]
    fn detects_current_and_stale_refresh_generations() {
        assert!(is_current_refresh_generation(7, 7));
        assert!(!is_current_refresh_generation(8, 7));
    }
}
