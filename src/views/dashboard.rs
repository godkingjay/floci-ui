#![allow(
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::wildcard_imports
)]

use icondata::{
    LuActivity, LuBoxes, LuClock, LuGlobe, LuMapPin, LuRefreshCw, LuServer, LuShieldCheck, LuTag,
};
use leptos::{ev::MouseEvent, *};
use leptos_icons::Icon;
use serde_json::Value;

use crate::{
    components::{
        Button, ButtonSize, ButtonVariant, MetricTile, Panel, PanelHeader, ServiceCategoryBadge,
        StatusBadge, StatusTone,
    },
    models::{DashboardSnapshot, HealthSnapshot, ServiceDescriptor},
    routes::AppRoute,
    state::{AppStore, RemoteData},
};

use super::{category_icon, category_label};

#[component]
pub fn DashboardView(store: AppStore, on_refresh: Callback<MouseEvent>) -> impl IntoView {
    view! {
        <div class="view-stack">
            <section class="dashboard-metrics" aria-label="Runtime summary">
                {move || render_metric_tiles(store.health.get(), store.catalog.get())}
            </section>

            <section class="status-grid">
                {move || render_health_panel(store.health.get(), on_refresh)}
                {move || render_runtime_panel(store.catalog.get(), on_refresh)}
            </section>

            <section class="services-section" aria-labelledby="service-summary-title">
                <div class="section-heading section-heading-row">
                    <div>
                        <p class="eyebrow">"Service Catalog"</p>
                        <h2 id="service-summary-title">"Local AWS surface"</h2>
                    </div>
                    <a class="shell-text-link" href=AppRoute::Services.href()>"View all services"</a>
                </div>
                {move || render_service_summary(store.catalog.get())}
            </section>
        </div>
    }
}

fn render_metric_tiles(
    health: RemoteData<HealthSnapshot>,
    catalog: RemoteData<DashboardSnapshot>,
) -> View {
    let health_value = match health {
        RemoteData::Ready(snapshot) => {
            if snapshot.ok {
                snapshot.health_status
            } else {
                "Unavailable".to_owned()
            }
        }
        RemoteData::Failed(_) => "Failed".to_owned(),
        RemoteData::Loading => "Checking".to_owned(),
    };

    let services_value = match &catalog {
        RemoteData::Ready(snapshot) => snapshot.services.len().to_string(),
        RemoteData::Failed(_) => "0".to_owned(),
        RemoteData::Loading => "-".to_owned(),
    };
    let endpoint_value = match &catalog {
        RemoteData::Ready(snapshot) => snapshot.endpoint_host(),
        RemoteData::Failed(_) => "Unavailable".to_owned(),
        RemoteData::Loading => "Loading".to_owned(),
    };
    let region_value = match catalog {
        RemoteData::Ready(snapshot) => snapshot.region,
        RemoteData::Failed(_) => "Unavailable".to_owned(),
        RemoteData::Loading => "Loading".to_owned(),
    };

    view! {
        <MetricTile label="Health" value=health_value icon=LuActivity />
        <MetricTile label="Services" value=services_value detail="catalog entries" icon=LuBoxes />
        <MetricTile label="Endpoint" value=endpoint_value icon=LuGlobe />
        <MetricTile label="Region" value=region_value icon=LuMapPin />
    }
    .into_view()
}

fn render_health_panel(
    state: RemoteData<HealthSnapshot>,
    on_refresh: Callback<MouseEvent>,
) -> View {
    match state {
        RemoteData::Loading => view! {
            <Panel class="status-panel">
                <div class="panel-status-row">
                    <StatusBadge tone=StatusTone::Muted label="Checking" />
                    <span class="panel-icon" aria-hidden="true">
                        <Icon icon=LuActivity width="1em" height="1em" />
                    </span>
                </div>
                <PanelHeader title="Floci health" icon=LuActivity />
                <p class="muted-text">"Waiting for the native runtime command response."</p>
            </Panel>
        }
        .into_view(),
        RemoteData::Failed(message) => view! {
            <Panel class="status-panel">
                <div class="panel-status-row">
                    <StatusBadge tone=StatusTone::Down label="Unavailable" />
                    <span class="panel-icon danger" aria-hidden="true">
                        <Icon icon=LuActivity width="1em" height="1em" />
                    </span>
                </div>
                <PanelHeader title="Floci health" icon=LuActivity />
                <InlineRetry message=message on_refresh=on_refresh />
            </Panel>
        }
        .into_view(),
        RemoteData::Ready(snapshot) => {
            let status_label = if snapshot.ok {
                snapshot.health_status.clone()
            } else {
                "Unavailable".to_owned()
            };
            let status_tone = if snapshot.ok {
                StatusTone::Up
            } else {
                StatusTone::Down
            };
            let body = snapshot
                .body
                .as_ref()
                .map_or_else(|| "No JSON body returned.".to_owned(), pretty_json);
            let error = snapshot.error.unwrap_or_default();
            let version = snapshot
                .floci_version
                .unwrap_or_else(|| "Not reported".to_owned());

            view! {
                <Panel class="status-panel">
                    <div class="panel-status-row">
                        <StatusBadge tone=status_tone label=status_label />
                        <span class="panel-icon" aria-hidden="true">
                            <Icon icon=LuActivity width="1em" height="1em" />
                        </span>
                    </div>
                    <PanelHeader title="Floci health" icon=LuActivity />
                    <dl class="summary-list">
                        <div>
                            <dt>"Endpoint"</dt>
                            <dd>{snapshot.url}</dd>
                        </div>
                        <div>
                            <dt>"HTTP status"</dt>
                            <dd>{snapshot.status.map_or_else(|| "None".to_owned(), |status| status.to_string())}</dd>
                        </div>
                        <div>
                            <dt>"Version"</dt>
                            <dd>{version}</dd>
                        </div>
                    </dl>
                    <pre class="json-preview">{body}</pre>
                    <p class="error-text">{error}</p>
                </Panel>
            }
            .into_view()
        }
    }
}

fn render_runtime_panel(
    state: RemoteData<DashboardSnapshot>,
    on_refresh: Callback<MouseEvent>,
) -> View {
    match state {
        RemoteData::Loading => view! {
            <Panel class="status-panel compact">
                <div class="panel-status-row">
                    <StatusBadge tone=StatusTone::Muted label="Loading" />
                    <span class="panel-icon" aria-hidden="true">
                        <Icon icon=LuServer width="1em" height="1em" />
                    </span>
                </div>
                <PanelHeader title="Runtime" icon=LuServer />
                <p class="muted-text">"Reading local emulator configuration."</p>
            </Panel>
        }
        .into_view(),
        RemoteData::Failed(message) => view! {
            <Panel class="status-panel compact">
                <div class="panel-status-row">
                    <StatusBadge tone=StatusTone::Down label="Blocked" />
                    <span class="panel-icon danger" aria-hidden="true">
                        <Icon icon=LuServer width="1em" height="1em" />
                    </span>
                </div>
                <PanelHeader title="Runtime" icon=LuServer />
                <InlineRetry message=message on_refresh=on_refresh />
            </Panel>
        }
        .into_view(),
        RemoteData::Ready(snapshot) => view! {
            <Panel class="status-panel compact">
                <div class="panel-status-row">
                    <StatusBadge tone=StatusTone::Neutral label="Configured" />
                    <span class="panel-icon" aria-hidden="true">
                        <Icon icon=LuServer width="1em" height="1em" />
                    </span>
                </div>
                <PanelHeader title="Runtime" icon=LuServer />
                <div class="data-grid runtime-data-grid">
                    <MetricTile label="Region" value=snapshot.region.clone() icon=LuMapPin />
                    <MetricTile
                        label="Credentials"
                        value=snapshot.credentials_status.clone()
                        icon=LuShieldCheck
                    />
                    <MetricTile
                        label="Last refreshed"
                        value=snapshot.last_refreshed_at.clone()
                        icon=LuClock
                    />
                </div>
                <dl class="summary-list">
                    <div>
                        <dt>"Endpoint"</dt>
                        <dd>{snapshot.endpoint_url}</dd>
                    </div>
                    <div>
                        <dt>"Access key"</dt>
                        <dd>{snapshot.access_key_id}</dd>
                    </div>
                </dl>
            </Panel>
        }
        .into_view(),
    }
}

fn render_service_summary(state: RemoteData<DashboardSnapshot>) -> View {
    match state {
        RemoteData::Loading => view! {
            <div class="service-grid">
                <article class="service-card">
                    <h3>"Loading services"</h3>
                    <p>"The service catalog is being loaded from the Tauri backend."</p>
                </article>
            </div>
        }
        .into_view(),
        RemoteData::Failed(message) => view! {
            <div class="service-grid">
                <article class="service-card">
                    <h3>"Catalog unavailable"</h3>
                    <p>{message}</p>
                </article>
            </div>
        }
        .into_view(),
        RemoteData::Ready(snapshot) => view! {
            <div class="service-grid summary-service-grid">
                {snapshot.services.into_iter().take(6).map(summary_service_card).collect_view()}
            </div>
        }
        .into_view(),
    }
}

fn summary_service_card(service: ServiceDescriptor) -> View {
    let href = AppRoute::ServiceDetail {
        key: service.key.clone(),
    }
    .href();

    view! {
        <a class="service-card service-card-link" href=href>
            <div class="service-card-header">
                <span class="service-icon" aria-hidden="true">
                    <Icon icon=category_icon(service.category) width="1.05em" height="1.05em" />
                </span>
                <h3>{service.label}</h3>
                <ServiceCategoryBadge category=service.category />
            </div>
            <p>{service.description}</p>
            <div class="toolbar-row">
                <code>{service.key}</code>
                <span class="resource-count icon-label">
                    <Icon icon=LuTag width="1em" height="1em" />
                    {category_label(service.category)}
                </span>
            </div>
        </a>
    }
    .into_view()
}

#[component]
fn InlineRetry(message: String, on_refresh: Callback<MouseEvent>) -> impl IntoView {
    view! {
        <div class="inline-error">
            <p>{message}</p>
            <div class="inline-error-actions">
                <Button
                    variant=ButtonVariant::Secondary
                    size=ButtonSize::Small
                    icon=view! { <Icon icon=LuRefreshCw width="1em" height="1em" /> }.into_view()
                    on_click=on_refresh
                >
                    "Retry"
                </Button>
            </div>
        </div>
    }
}

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value)
        .unwrap_or_else(|err| format!("Could not format JSON body: {err}"))
}
