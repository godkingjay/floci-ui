#![allow(
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::wildcard_imports
)]

use icondata::{
    LuAlertTriangle, LuArrowUpRight, LuFilter, LuFolder, LuLoader2, LuPackageSearch, LuRefreshCw,
};
use leptos::{ev::MouseEvent, *};
use leptos_icons::Icon;

use crate::{
    components::{
        Button, ButtonSize, ButtonVariant, EmptyState, Panel, PanelHeader, SearchInput,
        SelectInput, SelectOption, ServiceCategoryBadge, StatusBadge, StatusTone,
    },
    models::{DashboardSnapshot, ServiceDescriptor},
    routes::AppRoute,
    service_management::{
        models::ServiceSupportLevel, views::service_overview::ServiceOverviewView,
    },
    state::{AppStore, RemoteData},
};

use super::category_icon;

#[component]
pub fn ServicesView(store: AppStore, on_refresh: Callback<MouseEvent>) -> impl IntoView {
    let (query, set_query) = create_signal(String::new());
    let (domain_filter, set_domain_filter) = create_signal("all".to_owned());

    view! {
        <div class="view-stack">
            <section class="section-heading">
                <p class="eyebrow">"Service Catalog"</p>
                <h2>"Floci services"</h2>
            </section>
            {move || {
                render_services(
                    store.catalog.get(),
                    on_refresh,
                    query,
                    set_query,
                    domain_filter,
                    set_domain_filter,
                )
            }}
        </div>
    }
}

#[component]
pub fn ServiceDetailView(
    store: AppStore,
    service_key: String,
    on_refresh: Callback<MouseEvent>,
) -> impl IntoView {
    let cached_detail = create_rw_signal(None::<(ServiceDescriptor, DashboardSnapshot)>);
    let lookup_key = service_key.clone();

    create_effect(move |_| {
        if cached_detail.get_untracked().is_some() {
            return;
        }

        let RemoteData::Ready(snapshot) = store.catalog.get() else {
            return;
        };

        let service = snapshot
            .services
            .iter()
            .find(|service| service.key == lookup_key)
            .cloned();

        if let Some(service) = service {
            cached_detail.set(Some((service, snapshot)));
        }
    });

    view! {
        <div class="view-stack">
            {move || {
                cached_detail.get().map_or_else(
                    || render_service_detail_status(store.catalog.get(), service_key.clone(), on_refresh),
                    |(service, snapshot)| render_loaded_service_detail(service, snapshot, on_refresh),
                )
            }}
        </div>
    }
}

fn render_services(
    state: RemoteData<DashboardSnapshot>,
    on_refresh: Callback<MouseEvent>,
    query: ReadSignal<String>,
    set_query: WriteSignal<String>,
    domain_filter: ReadSignal<String>,
    set_domain_filter: WriteSignal<String>,
) -> View {
    match state {
        RemoteData::Loading => view! {
            <div class="service-grid">
                <article class="service-card">
                    <div class="service-card-header">
                        <span class="service-icon" aria-hidden="true">
                            <Icon icon=LuLoader2 width="1.05em" height="1.05em" />
                        </span>
                        <h3>"Loading services"</h3>
                    </div>
                    <p>"The service catalog is being loaded from the Tauri backend."</p>
                </article>
            </div>
        }
        .into_view(),
        RemoteData::Failed(message) => view! {
            <Panel class="status-panel">
                <PanelHeader title="Catalog unavailable" icon=LuAlertTriangle />
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
            </Panel>
        }
        .into_view(),
        RemoteData::Ready(snapshot) => {
            render_service_catalog(snapshot, query, set_query, domain_filter, set_domain_filter)
        }
    }
}

fn render_service_catalog(
    snapshot: DashboardSnapshot,
    query: ReadSignal<String>,
    set_query: WriteSignal<String>,
    domain_filter: ReadSignal<String>,
    set_domain_filter: WriteSignal<String>,
) -> View {
    let services = snapshot.services;
    let total_services = services.len();
    let domain_options = domain_filter_options(&services);
    let services_for_view = services.clone();
    let current_query = query.get_untracked();
    let current_domain_filter = domain_filter.get_untracked();

    view! {
        <div class="service-catalog-stack">
            <div class="service-catalog-controls">
                <SearchInput
                    id="service-search"
                    value=current_query
                    placeholder="Search services, resource kinds, or operations"
                    on_input=Callback::new(move |value| set_query.set(value))
                />
                <SelectInput
                    id="service-domain-filter"
                    label="Domain"
                    options=domain_options
                    value=current_domain_filter
                    on_change=Callback::new(move |value| set_domain_filter.set(value))
                />
                <span class="resource-count icon-label">
                    <Icon icon=LuFilter width="1em" height="1em" />
                    {format!("{total_services} services")}
                </span>
            </div>
            {move || {
                let filtered = filter_services(
                    &services_for_view,
                    &query.get(),
                    &domain_filter.get(),
                );

                if filtered.is_empty() {
                    view! {
                        <EmptyState
                            title="No services found"
                            message="Adjust the search or domain filter."
                        />
                    }
                    .into_view()
                } else {
                    view! {
                        <div class="service-grid service-catalog-grid">
                            {filtered.into_iter().map(service_card).collect_view()}
                        </div>
                    }
                    .into_view()
                }
            }}
        </div>
    }
    .into_view()
}

fn render_service_detail_status(
    state: RemoteData<DashboardSnapshot>,
    service_key: String,
    on_refresh: Callback<MouseEvent>,
) -> View {
    match state {
        RemoteData::Loading => view! {
            <Panel class="status-panel">
                <PanelHeader title="Loading service" icon=LuLoader2 />
                <p class="muted-text">"Reading the service catalog."</p>
            </Panel>
        }
        .into_view(),
        RemoteData::Failed(message) => view! {
            <Panel class="status-panel">
                <PanelHeader title="Service unavailable" icon=LuAlertTriangle />
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
            </Panel>
        }
        .into_view(),
        RemoteData::Ready(snapshot) => {
            if snapshot
                .services
                .iter()
                .any(|service| service.key == service_key)
            {
                view! {
                    <Panel class="status-panel">
                        <PanelHeader title="Preparing service" icon=LuLoader2 />
                        <p class="muted-text">"Preparing the service workspace."</p>
                    </Panel>
                }
                .into_view()
            } else {
                view! {
                    <EmptyState
                        title="Service not found"
                        message=format!("No service catalog entry exists for `{service_key}`.")
                        action=view! {
                            <a class="shell-text-link" href=AppRoute::Services.href()>"Back to services"</a>
                        }.into_view()
                    />
                }
                .into_view()
            }
        }
    }
}

fn render_loaded_service_detail(
    service: ServiceDescriptor,
    snapshot: DashboardSnapshot,
    on_refresh: Callback<MouseEvent>,
) -> View {
    view! {
        <ServiceOverviewView
            service=service
            snapshot=snapshot
            on_catalog_refresh=on_refresh
        />
    }
    .into_view()
}

fn service_card(service: ServiceDescriptor) -> View {
    let href = AppRoute::ServiceDetail {
        key: service.key.clone(),
    }
    .href();
    let resource_summary = resource_summary(&service);

    view! {
        <a class="service-card service-card-link" href=href>
            <div class="service-card-header">
                <span class="service-icon" aria-hidden="true">
                    <Icon icon=category_icon(service.category) width="1.05em" height="1.05em" />
                </span>
                <h3>{service.label}</h3>
                <ServiceCategoryBadge category=service.category />
            </div>
            <p class="service-card-description">{service.description}</p>
            <div class="service-card-meta">
                <StatusBadge
                    tone=support_tone(service.support_level)
                    label=service.support_level.label()
                />
                <span class="domain-pill">
                    <Icon icon=LuFolder width="1em" height="1em" />
                    {service.domain_epic}
                </span>
                <span class="resource-kind-chip">
                    <Icon icon=LuPackageSearch width="1em" height="1em" />
                    {resource_summary}
                </span>
            </div>
            <div class="service-card-footer">
                <code>{service.key}</code>
                <span class="service-card-open">
                    <Icon icon=LuArrowUpRight width="1em" height="1em" />
                    "Open"
                </span>
            </div>
        </a>
    }
    .into_view()
}

fn filter_services(
    services: &[ServiceDescriptor],
    query: &str,
    domain_filter: &str,
) -> Vec<ServiceDescriptor> {
    let query = query.trim().to_ascii_lowercase();

    services
        .iter()
        .filter(|service| domain_filter == "all" || service.domain_epic == domain_filter)
        .filter(|service| {
            if query.is_empty() {
                return true;
            }

            let resource_match = service
                .primary_resource_kinds
                .iter()
                .chain(service.safe_operations.iter())
                .any(|value| value.to_ascii_lowercase().contains(&query));

            service.key.contains(&query)
                || service.label.to_ascii_lowercase().contains(&query)
                || service.description.to_ascii_lowercase().contains(&query)
                || service.domain_epic.to_ascii_lowercase().contains(&query)
                || resource_match
        })
        .cloned()
        .collect()
}

fn domain_filter_options(services: &[ServiceDescriptor]) -> Vec<SelectOption> {
    let mut domains = services
        .iter()
        .map(|service| service.domain_epic.clone())
        .collect::<std::collections::BTreeSet<_>>();
    domains.remove("");

    std::iter::once(SelectOption {
        label: "All domains".to_owned(),
        value: "all".to_owned(),
    })
    .chain(domains.into_iter().map(|domain| SelectOption {
        label: domain.clone(),
        value: domain,
    }))
    .collect()
}

fn resource_summary(service: &ServiceDescriptor) -> String {
    match service.primary_resource_kinds.len() {
        0 => "No resources".to_owned(),
        1 => "1 resource kind".to_owned(),
        count => format!("{count} resource kinds"),
    }
}

fn support_tone(level: ServiceSupportLevel) -> StatusTone {
    match level {
        ServiceSupportLevel::Managed => StatusTone::Up,
        ServiceSupportLevel::ReadOnly => StatusTone::Muted,
        ServiceSupportLevel::Unsupported => StatusTone::Neutral,
    }
}
