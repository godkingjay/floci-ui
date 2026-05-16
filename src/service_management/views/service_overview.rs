#![allow(
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::wildcard_imports
)]

use icondata::{
    LuAlertTriangle, LuClock, LuFileJson, LuGlobe, LuKeyRound, LuMapPin, LuPackageSearch,
    LuRefreshCw,
};
use leptos::{ev::MouseEvent, *};
use leptos_icons::Icon;
use wasm_bindgen_futures::spawn_local;

use crate::{
    components::{
        Button, ButtonSize, ButtonVariant, InlineNotice, MetricTile, NoticeTone, Panel,
        PanelHeader, StatusBadge, StatusTone,
    },
    models::{DashboardSnapshot, ServiceDescriptor},
    routes::AppRoute,
    service_management::{
        client,
        domains::is_domain_service,
        models::{ResourceDetail, ResourceSummary, ServiceInventory, ServiceSupportLevel},
        views::{
            action_dialog::ActionDialog,
            data_storage::DataStorageServiceView,
            resource_detail::{JsonInspectorPanel, ResourceDetailDialog},
            resource_table::ResourceTable,
        },
    },
    state::RemoteData,
};

#[component]
pub fn ServiceOverviewView(
    service: ServiceDescriptor,
    snapshot: DashboardSnapshot,
    on_catalog_refresh: Callback<MouseEvent>,
) -> impl IntoView {
    if is_domain_service(&service.key) {
        return view! {
            <DataStorageServiceView
                service=service
                snapshot=snapshot
                on_catalog_refresh=on_catalog_refresh
            />
        }
        .into_view();
    }

    let service_key = service.key.clone();
    let service_key_for_load = service_key.clone();
    let (inventory, set_inventory) = create_signal(RemoteData::<ServiceInventory>::Loading);
    let (selected_tab, set_selected_tab) = create_signal(String::new());
    let (_selected_resource, set_selected_resource) = create_signal(None::<ResourceSummary>);
    let (detail_resource, set_detail_resource) = create_signal(None::<ResourceSummary>);
    let (detail_state, set_detail_state) = create_signal(RemoteData::<ResourceDetail>::Loading);

    spawn_local(async move {
        set_inventory.set(remote_result(
            client::service_inventory(service_key_for_load).await,
        ));
    });

    let refresh_inventory = {
        let service_key = service_key.clone();
        move || {
            set_inventory.set(RemoteData::Loading);
            set_selected_resource.set(None);
            set_detail_resource.set(None);
            let service_key = service_key.clone();

            spawn_local(async move {
                set_inventory.set(remote_result(client::service_inventory(service_key).await));
            });
        }
    };

    view! {
        <div class="service-management-view">
            <section class="service-management-hero">
                <div>
                    <p class="eyebrow">{service.domain_epic.clone()}</p>
                    <h2>{service.label.clone()}</h2>
                    <p>{service.description.clone()}</p>
                    <div class="chip-row">
                        <StatusBadge
                            tone=support_tone(service.support_level)
                            label=service.support_level.label()
                        />
                        <code>{service.key.clone()}</code>
                    </div>
                </div>
                <div class="service-management-actions">
                    <Button
                        variant=ButtonVariant::Secondary
                        size=ButtonSize::Small
                        icon=view! { <Icon icon=LuRefreshCw width="1em" height="1em" /> }.into_view()
                        on_click=Callback::new(move |event| on_catalog_refresh.call(event))
                    >
                        "Refresh catalog"
                    </Button>
                    <Button
                        variant=ButtonVariant::Primary
                        size=ButtonSize::Small
                        icon=view! { <Icon icon=LuRefreshCw width="1em" height="1em" /> }.into_view()
                        loading=Signal::derive(move || inventory.get().is_loading())
                        on_click=Callback::new(move |_| refresh_inventory())
                    >
                        "Refresh inventory"
                    </Button>
                </div>
            </section>

            {move || render_inventory(
                service.clone(),
                snapshot.clone(),
                inventory.get(),
                selected_tab,
                set_selected_tab,
                set_selected_resource,
                detail_resource,
                set_detail_resource,
                detail_state,
                set_detail_state,
            )}
        </div>
    }
    .into_view()
}

fn render_inventory(
    service: ServiceDescriptor,
    snapshot: DashboardSnapshot,
    state: RemoteData<ServiceInventory>,
    selected_tab: ReadSignal<String>,
    set_selected_tab: WriteSignal<String>,
    set_selected_resource: WriteSignal<Option<ResourceSummary>>,
    detail_resource: ReadSignal<Option<ResourceSummary>>,
    set_detail_resource: WriteSignal<Option<ResourceSummary>>,
    detail_state: ReadSignal<RemoteData<ResourceDetail>>,
    set_detail_state: WriteSignal<RemoteData<ResourceDetail>>,
) -> View {
    match state {
        RemoteData::Loading => view! {
            <Panel class="status-panel">
                <PanelHeader title="Loading inventory" icon=LuPackageSearch />
                <p class="muted-text">"Reading local service inventory through the Tauri backend."</p>
            </Panel>
        }
        .into_view(),
        RemoteData::Failed(message) => view! {
            <Panel class="status-panel">
                <PanelHeader title="Inventory unavailable" icon=LuAlertTriangle />
                <div class="inline-error">
                    <p>{message}</p>
                </div>
            </Panel>
        }
        .into_view(),
        RemoteData::Ready(inventory) => {
            let tab_key = active_tab_key(&inventory, selected_tab.get());
            let active_tab = inventory
                .tabs
                .iter()
                .find(|tab| tab.key == tab_key)
                .cloned();
            let resources = active_tab.as_ref().map_or_else(Vec::new, |tab| {
                inventory
                    .resources
                    .iter()
                    .filter(|resource| tab.kinds.iter().any(|kind| kind == &resource.kind))
                    .cloned()
                    .collect::<Vec<_>>()
            });
            let inventory_json = serde_json::to_string_pretty(&inventory)
                .unwrap_or_else(|_| "{}".to_owned());
            let service_key = service.key.clone();
            let on_view_resource = Callback::new(move |resource: ResourceSummary| {
                set_selected_resource.set(Some(resource.clone()));
                set_detail_resource.set(Some(resource.clone()));
                set_detail_state.set(RemoteData::Loading);
                let service_key = service_key.clone();
                let resource_id = resource.id.clone();

                spawn_local(async move {
                    set_detail_state.set(remote_result(
                        client::service_resource_detail(service_key, resource_id).await,
                    ));
                });
            });
            let endpoint_host = snapshot.endpoint_host();
            let region = snapshot.region.clone();
            let credentials_status = snapshot.credentials_status.clone();
            let last_refreshed_at = snapshot.last_refreshed_at.clone();
            let inventory_refreshed_at = inventory.refreshed_at.clone();
            let service_label = inventory.service_label.clone();
            let support_level = inventory.support_level;
            let resource_kind_count = inventory.tabs.len();
            let unsupported_operations = inventory.unsupported_operations.clone();

            view! {
                <div class="service-management-grid">
                    <Panel class="status-panel service-inventory-panel">
                        <div class="panel-status-row">
                            <StatusBadge
                                tone=support_tone(support_level)
                                label=support_level.label()
                            />
                            <span class="resource-count">
                                {format!("{resource_kind_count} resource kinds")}
                            </span>
                        </div>
                        <PanelHeader
                            title=format!("{service_label} inventory")
                            icon=LuPackageSearch
                            description=format!("Endpoint {endpoint_host} · refreshed {inventory_refreshed_at}")
                        />
                        {matches!(support_level, ServiceSupportLevel::Unsupported).then(|| view! {
                            <InlineNotice tone=NoticeTone::Warning title="Unsupported adapter">
                                "This service is registered in the catalog and route system, but the local adapter is pending."
                            </InlineNotice>
                        })}
                        {render_tabs(inventory.tabs.clone(), tab_key, set_selected_tab)}
                        <ResourceTable
                            resources=resources
                            active_tab=active_tab
                            on_view_resource=on_view_resource
                        />
                    </Panel>

                    <div class="service-side-stack">
                        <ActionDialog
                            service_label=service.label.clone()
                            support_level=support_level
                            safe_operations=service.safe_operations.clone()
                            unsupported_operations=unsupported_operations
                        />
                        <Panel class="status-panel compact">
                            <PanelHeader title="Runtime context" icon=LuGlobe />
                            <div class="data-grid runtime-data-grid">
                                <MetricTile label="Region" value=region icon=LuMapPin />
                                <MetricTile label="Credentials" value=credentials_status icon=LuKeyRound />
                                <MetricTile label="Last refreshed" value=last_refreshed_at icon=LuClock />
                            </div>
                            <a class="shell-text-link" href=AppRoute::Services.href()>"Back to services"</a>
                        </Panel>
                        <JsonInspectorPanel title="JSON inspector" json=inventory_json icon=LuFileJson />
                    </div>
                    {move || view! {
                        <ResourceDetailDialog
                            open=detail_resource.get().is_some()
                            selected=detail_resource.get()
                            detail=detail_state.get()
                            on_close=Callback::new(move |_| set_detail_resource.set(None))
                        />
                    }}
                </div>
            }
            .into_view()
        }
    }
}

fn render_tabs(
    tabs: Vec<crate::service_management::models::ResourceTab>,
    active_key: String,
    set_selected_tab: WriteSignal<String>,
) -> View {
    if tabs.is_empty() {
        return view! {
            <div class="resource-tabs empty">
                <span>"No resource kinds"</span>
            </div>
        }
        .into_view();
    }

    view! {
        <div class="resource-tabs" role="tablist">
            {tabs.into_iter().map(|tab| {
                let is_active = tab.key == active_key;
                let tab_key = tab.key.clone();
                view! {
                    <button
                        type="button"
                        class=("active", is_active)
                        role="tab"
                        aria-selected=is_active.to_string()
                        on:click=move |_| set_selected_tab.set(tab_key.clone())
                    >
                        {tab.label}
                    </button>
                }
            }).collect_view()}
        </div>
    }
    .into_view()
}

fn active_tab_key(inventory: &ServiceInventory, selected_tab: String) -> String {
    if inventory.tabs.iter().any(|tab| tab.key == selected_tab) {
        return selected_tab;
    }

    inventory
        .tabs
        .first()
        .map_or_else(String::new, |tab| tab.key.clone())
}

fn support_tone(level: ServiceSupportLevel) -> StatusTone {
    match level {
        ServiceSupportLevel::Managed => StatusTone::Up,
        ServiceSupportLevel::ReadOnly => StatusTone::Muted,
        ServiceSupportLevel::Unsupported => StatusTone::Neutral,
    }
}

fn remote_result<T>(result: Result<T, String>) -> RemoteData<T> {
    match result {
        Ok(value) => RemoteData::Ready(value),
        Err(message) => RemoteData::Failed(message),
    }
}
