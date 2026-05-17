#![allow(
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::wildcard_imports
)]

use icondata::{
    Icon as IconData, LuBoxes, LuCloudCog, LuLayoutDashboard, LuMoon, LuRefreshCw, LuSettings,
    LuSun,
};
use leptos::{ev::MouseEvent, *};
use leptos_icons::Icon;

use crate::{
    components::{
        Button, ButtonSize, ButtonVariant, StatusBadge, StatusTone, UpdateDialog, UpdateNotice,
    },
    models::{DashboardSnapshot, HealthSnapshot},
    routes::AppRoute,
    state::{AppStore, RemoteData},
    theme::ThemeMode,
};

#[component]
pub fn AppShell(
    store: AppStore,
    on_refresh: Callback<MouseEvent>,
    on_toggle_theme: Callback<MouseEvent>,
    children: Children,
) -> impl IntoView {
    view! {
        <main class="runtime-shell antialiased">
            <ShellSidebar route=store.route />
            <section class="shell-main">
                <ShellHeader
                    store=store
                    on_refresh=on_refresh
                    on_toggle_theme=on_toggle_theme
                />
                <UpdateNotice store=store />
                <div class="compact-nav">
                    <ShellSidebar route=store.route compact=true />
                </div>
                <div class="shell-content">{children()}</div>
                <HealthLiveRegion health=store.health />
                <UpdateDialog store=store />
            </section>
        </main>
    }
}

#[component]
pub fn ShellSidebar(route: ReadSignal<AppRoute>, #[prop(optional)] compact: bool) -> impl IntoView {
    let class_name = if compact {
        "shell-sidebar compact"
    } else {
        "shell-sidebar"
    };

    view! {
        <aside class=class_name aria-label="Primary navigation">
            {(!compact).then(|| view! {
                <a class="sidebar-brand" href=AppRoute::Dashboard.href() aria-label="Floci UI dashboard">
                    <span class="brand-mark" aria-hidden="true">
                        <Icon icon=LuCloudCog width="1.05em" height="1.05em" />
                    </span>
                    <span>
                        <strong>"Floci UI"</strong>
                        <small>"AWS Local Emulator"</small>
                    </span>
                </a>
            })}
            <nav class="shell-nav">
                {move || nav_items(route.get())
                    .into_iter()
                    .map(|item| {
                        let class_name = if item.active {
                            "shell-nav-link active"
                        } else {
                            "shell-nav-link"
                        };

                        view! {
                            <a
                                class=class_name
                                href=item.href
                                aria-current=item.active.then_some("page")
                            >
                                <span class="shell-nav-icon" aria-hidden="true">
                                    <Icon icon=item.icon width="1em" height="1em" />
                                </span>
                                <span>{item.label}</span>
                            </a>
                        }
                    })
                    .collect_view()}
            </nav>
        </aside>
    }
}

#[component]
pub fn ShellHeader(
    store: AppStore,
    on_refresh: Callback<MouseEvent>,
    on_toggle_theme: Callback<MouseEvent>,
) -> impl IntoView {
    view! {
        <header class="shell-header">
            <div class="brand-lockup">
                <span class="brand-mark" aria-hidden="true">
                    <Icon icon=LuCloudCog width="1.15em" height="1.15em" />
                </span>
                <div>
                    <p class="eyebrow">"Floci UI"</p>
                    <h1 class="text-page-title">
                        {move || store.route.get().label()}
                    </h1>
                </div>
            </div>
            <div class="shell-header-meta">
                <EndpointIndicator health=store.health catalog=store.catalog />
                <div class="top-bar-actions">
                    <Button
                        variant=ButtonVariant::Secondary
                        size=ButtonSize::Medium
                        icon=view! {
                            {move || match store.theme_mode.get() {
                                ThemeMode::Dark => view! {
                                    <Icon icon=LuSun width="1em" height="1em" />
                                }.into_view(),
                                ThemeMode::Light => view! {
                                    <Icon icon=LuMoon width="1em" height="1em" />
                                }.into_view(),
                            }}
                        }.into_view()
                        on_click=on_toggle_theme
                    >
                        {move || match store.theme_mode.get() {
                            ThemeMode::Dark => "Light",
                            ThemeMode::Light => "Dark",
                        }}
                    </Button>
                    <RefreshControl refresh_state=store.refresh on_refresh=on_refresh />
                </div>
            </div>
        </header>
    }
}

#[component]
pub fn EndpointIndicator(
    health: ReadSignal<RemoteData<HealthSnapshot>>,
    catalog: ReadSignal<RemoteData<DashboardSnapshot>>,
) -> impl IntoView {
    view! {
        <div class="endpoint-indicator">
            {move || {
                let health = health.get();
                let (tone, label) = match &health {
                    RemoteData::Ready(snapshot) if snapshot.ok => (StatusTone::Up, "Healthy"),
                    RemoteData::Ready(_) | RemoteData::Failed(_) => (StatusTone::Down, "Unavailable"),
                    RemoteData::Loading => (StatusTone::Muted, "Checking"),
                };

                view! { <StatusBadge tone=tone label=label /> }
            }}
            <span class="endpoint-meta">
                {move || match catalog.get() {
                    RemoteData::Ready(snapshot) => format!(
                        "{} · {}",
                        snapshot.endpoint_host(),
                        snapshot.region
                    ),
                    RemoteData::Failed(_) => "Runtime unavailable".to_owned(),
                    RemoteData::Loading => "Reading runtime".to_owned(),
                }}
            </span>
        </div>
    }
}

#[component]
pub fn RefreshControl(
    refresh_state: ReadSignal<crate::state::RefreshState>,
    on_refresh: Callback<MouseEvent>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class="ui-button ui-button-primary ui-button-medium"
            disabled=move || refresh_state.get().is_loading()
            aria-busy=move || refresh_state.get().is_loading().to_string()
            on:click=move |event| {
                if !refresh_state.get_untracked().is_loading() {
                    on_refresh.call(event);
                }
            }
        >
            <span class="ui-button-icon">
                <Icon icon=LuRefreshCw width="1em" height="1em" />
            </span>
            <span class="ui-button-content">"Refresh"</span>
            {move || refresh_state.get().is_loading().then(|| {
                view! { <span class="ui-button-spinner" aria-hidden="true"></span> }
            })}
        </button>
    }
}

#[component]
fn HealthLiveRegion(health: ReadSignal<RemoteData<HealthSnapshot>>) -> impl IntoView {
    view! {
        <div class="sr-only" aria-live="polite">
            {move || match health.get() {
                RemoteData::Ready(snapshot) if snapshot.ok => {
                    format!("Floci health is {}.", snapshot.health_status)
                }
                RemoteData::Ready(snapshot) => {
                    format!("Floci health is unavailable: {}.", snapshot.health_status)
                }
                RemoteData::Failed(message) => format!("Floci health check failed: {message}."),
                RemoteData::Loading => "Checking Floci health.".to_owned(),
            }}
        </div>
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ShellNavItem {
    label: &'static str,
    icon: IconData,
    href: String,
    active: bool,
}

fn nav_items(route: AppRoute) -> Vec<ShellNavItem> {
    vec![
        ShellNavItem {
            label: "Dashboard",
            icon: LuLayoutDashboard,
            href: AppRoute::Dashboard.href(),
            active: route == AppRoute::Dashboard,
        },
        ShellNavItem {
            label: "Services",
            icon: LuBoxes,
            href: AppRoute::Services.href(),
            active: route.is_services_section(),
        },
        ShellNavItem {
            label: "Settings",
            icon: LuSettings,
            href: AppRoute::Settings.href(),
            active: route == AppRoute::Settings,
        },
    ]
}
