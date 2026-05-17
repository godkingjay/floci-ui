#![allow(clippy::must_use_candidate, clippy::wildcard_imports)]

use leptos::*;

use crate::{
    components::AppShell,
    routes::{AppRoute, install_hash_route_listener},
    state::AppStore,
    views::{DashboardView, ServiceDetailView, ServicesView, SettingsView},
};

#[component]
pub fn App() -> impl IntoView {
    let store = AppStore::create();
    let on_refresh = Callback::new(move |_| store.refresh_dashboard());
    let on_toggle_theme = Callback::new(move |_| store.toggle_theme());

    install_hash_route_listener(store.set_route);
    store.refresh_dashboard();
    store.check_for_updates();

    view! {
        <AppShell store=store on_refresh=on_refresh on_toggle_theme=on_toggle_theme>
            {move || match store.route.get() {
                AppRoute::Dashboard => view! {
                    <DashboardView store=store on_refresh=on_refresh />
                }.into_view(),
                AppRoute::Services => view! {
                    <ServicesView store=store on_refresh=on_refresh />
                }.into_view(),
                AppRoute::ServiceDetail { key } => view! {
                    <ServiceDetailView store=store service_key=key on_refresh=on_refresh />
                }.into_view(),
                AppRoute::Settings => view! {
                    <SettingsView store=store />
                }.into_view(),
            }}
        </AppShell>
    }
}
