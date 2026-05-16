use leptos::*;
use wasm_bindgen_futures::spawn_local;

use crate::{
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
    pub route: ReadSignal<AppRoute>,
    pub set_route: WriteSignal<AppRoute>,
    pub theme_preference: ReadSignal<ThemePreference>,
    pub set_theme_preference: WriteSignal<ThemePreference>,
    pub theme_mode: ReadSignal<ThemeMode>,
    pub set_theme_mode: WriteSignal<ThemeMode>,
}

impl AppStore {
    pub fn create() -> Self {
        let (health, set_health) = create_signal(RemoteData::Loading);
        let (catalog, set_catalog) = create_signal(RemoteData::Loading);
        let (refresh, set_refresh) = create_signal(RefreshState::default());
        let (route, set_route) = create_signal(AppRoute::current());
        let initial_preference = read_theme_preference();
        let (theme_preference, set_theme_preference) = create_signal(initial_preference);
        let (theme_mode, set_theme_mode) = create_signal(initial_preference.resolved_mode());

        Self {
            health,
            set_health,
            catalog,
            set_catalog,
            refresh,
            set_refresh,
            route,
            set_route,
            theme_preference,
            set_theme_preference,
            theme_mode,
            set_theme_mode,
        }
    }

    pub fn refresh_dashboard(self) {
        self.set_health.set(RemoteData::Loading);
        self.set_catalog.set(RemoteData::Loading);
        self.set_refresh.set(RefreshState {
            health_loading: true,
            catalog_loading: true,
        });

        spawn_local(async move {
            let health = invoke_command::<HealthSnapshot>("floci_health").await;
            self.set_health.set(remote_result(health));
            self.set_refresh.update(|state| {
                state.health_loading = false;
            });

            let catalog = invoke_command::<DashboardSnapshot>("service_catalog").await;
            self.set_catalog.set(remote_result(catalog));
            self.set_refresh.update(|state| {
                state.catalog_loading = false;
            });
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
