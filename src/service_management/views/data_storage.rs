#![allow(clippy::must_use_candidate, clippy::wildcard_imports)]

use std::rc::Rc;

use icondata::{
    Icon as IconData, LuAlertTriangle, LuCircleSlash, LuFileJson, LuPackageSearch, LuPlus,
    LuRefreshCw, LuTrash2,
};
use leptos::{
    ev::MouseEvent,
    html::{Input, Textarea},
    *,
};
use leptos_icons::Icon;
use serde_json::{Value, json};
use wasm_bindgen_futures::spawn_local;

use crate::{
    components::{
        Button, ButtonSize, ButtonVariant, ConfirmDialog, Dialog, InlineNotice, NoticeTone, Panel,
        PanelHeader, StatusBadge, StatusTone,
    },
    models::{DashboardSnapshot, ServiceDescriptor},
    service_management::{
        client,
        domains::{ActionKind, ServiceActionDefinition, ServiceDomainDefinition, definition},
        models::{
            ActionResult, ResourceDetail, ResourceSummary, ServiceActionRequest, ServiceInventory,
        },
        views::{
            resource_detail::{JsonInspectorPanel, ResourceDetailDialog},
            resource_table::{ResourceRowAction, ResourceRowActionTone, ResourceTable},
        },
    },
    state::RemoteData,
};

#[component]
pub fn DataStorageServiceView(
    service: ServiceDescriptor,
    snapshot: DashboardSnapshot,
    on_catalog_refresh: Callback<MouseEvent>,
) -> impl IntoView {
    let service_key = service.key.clone();
    let service_label = service.label.clone();
    let service_description = service.description.clone();
    let domain = definition(&service_key).expect("service domain definition should exist");

    let (inventory, set_inventory) = create_signal(RemoteData::<ServiceInventory>::Loading);
    let (selected_tab, set_selected_tab) = create_signal(String::new());
    let (_selected_resource, set_selected_resource) = create_signal(None::<ResourceSummary>);
    let (detail_resource, set_detail_resource) = create_signal(None::<ResourceSummary>);
    let (detail_state, set_detail_state) = create_signal(RemoteData::<ResourceDetail>::Loading);
    let (delete_action, set_delete_action) =
        create_signal(None::<(ServiceActionDefinition, ResourceSummary)>);
    let (resource_action, set_resource_action) =
        create_signal(None::<(ServiceActionDefinition, ResourceSummary)>);
    let (active_action, set_active_action) = create_signal(None::<ServiceActionDefinition>);
    let (action_message, set_action_message) = create_signal(None::<String>);
    let (action_error, set_action_error) = create_signal(None::<String>);
    let (action_loading, set_action_loading) = create_signal(false);
    let primary_ref = create_node_ref::<Input>();
    let secondary_ref = create_node_ref::<Input>();
    let payload_ref = create_node_ref::<Textarea>();

    {
        let service_key = service_key.clone();
        spawn_local(async move {
            set_inventory.set(RemoteData::Loading);
            set_inventory.set(remote_result(client::service_inventory(service_key).await));
        });
    }

    let refresh_inventory = {
        let service_key = service_key.clone();
        move || {
            set_inventory.set(RemoteData::Loading);
            set_selected_resource.set(None);
            set_detail_resource.set(None);
            set_action_error.set(None);
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
                    <p class="eyebrow">{domain.domain_label}</p>
                    <h2>{service_label}</h2>
                    <p>{service_description}</p>
                    <p class="muted-text">{domain.summary}</p>
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
                service_key.clone(),
                domain,
                snapshot.clone(),
                inventory.get(),
                set_inventory,
                selected_tab,
                set_selected_tab,
                set_selected_resource,
                detail_resource,
                set_detail_resource,
                detail_state,
                set_detail_state,
                delete_action,
                set_delete_action,
                resource_action,
                set_resource_action,
                active_action,
                set_active_action,
                action_message,
                set_action_message,
                action_error,
                set_action_error,
                action_loading,
                set_action_loading,
                primary_ref,
                secondary_ref,
                payload_ref,
            )}
        </div>
    }
}

#[allow(clippy::too_many_arguments)]
fn render_inventory(
    service_key: String,
    domain: &'static ServiceDomainDefinition,
    snapshot: DashboardSnapshot,
    state: RemoteData<ServiceInventory>,
    set_inventory: WriteSignal<RemoteData<ServiceInventory>>,
    selected_tab: ReadSignal<String>,
    set_selected_tab: WriteSignal<String>,
    set_selected_resource: WriteSignal<Option<ResourceSummary>>,
    detail_resource: ReadSignal<Option<ResourceSummary>>,
    set_detail_resource: WriteSignal<Option<ResourceSummary>>,
    detail_state: ReadSignal<RemoteData<ResourceDetail>>,
    set_detail_state: WriteSignal<RemoteData<ResourceDetail>>,
    delete_action: ReadSignal<Option<(ServiceActionDefinition, ResourceSummary)>>,
    set_delete_action: WriteSignal<Option<(ServiceActionDefinition, ResourceSummary)>>,
    resource_action: ReadSignal<Option<(ServiceActionDefinition, ResourceSummary)>>,
    set_resource_action: WriteSignal<Option<(ServiceActionDefinition, ResourceSummary)>>,
    active_action: ReadSignal<Option<ServiceActionDefinition>>,
    set_active_action: WriteSignal<Option<ServiceActionDefinition>>,
    action_message: ReadSignal<Option<String>>,
    set_action_message: WriteSignal<Option<String>>,
    action_error: ReadSignal<Option<String>>,
    set_action_error: WriteSignal<Option<String>>,
    action_loading: ReadSignal<bool>,
    set_action_loading: WriteSignal<bool>,
    primary_ref: NodeRef<Input>,
    secondary_ref: NodeRef<Input>,
    payload_ref: NodeRef<Textarea>,
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
            let toolbar_service_key = service_key.clone();
            let detail_service_key = service_key.clone();
            let create_dialog_service_key = service_key.clone();
            let resource_dialog_service_key = service_key.clone();
            let delete_dialog_service_key = service_key.clone();
            let on_view_resource = Callback::new(move |resource: ResourceSummary| {
                set_selected_resource.set(Some(resource.clone()));
                set_detail_resource.set(Some(resource.clone()));
                set_detail_state.set(RemoteData::Loading);
                let service_key = detail_service_key.clone();
                let resource_id = resource.id.clone();

                spawn_local(async move {
                    set_detail_state.set(remote_result(
                        client::service_resource_detail(service_key, resource_id).await,
                    ));
                });
            });
            let row_actions = resource_row_actions(
                service_key.clone(),
                domain.actions,
                inventory.unsupported_operations.clone(),
                set_selected_resource,
                set_resource_action,
                set_delete_action,
                set_action_message,
                set_action_error,
                set_action_loading,
                set_inventory,
            );

            view! {
                <div class="service-management-grid">
                    <Panel class="status-panel service-inventory-panel">
                        <div class="panel-status-row">
                            <StatusBadge
                                tone=support_tone(inventory.support_level)
                                label=inventory.support_level.label()
                            />
                            <span class="resource-count">
                                {format!("{} resources", inventory.resources.len())}
                            </span>
                        </div>
                        <PanelHeader
                            title=format!("{} inventory", inventory.service_label)
                            icon=LuPackageSearch
                            description=format!("Endpoint {} · refreshed {}", snapshot.endpoint_host(), inventory.refreshed_at)
                        />

                        {render_action_toolbar(
                            toolbar_service_key,
                            domain.actions,
                            set_active_action,
                            set_action_message,
                            set_action_error,
                        )}
                        {render_action_feedback(action_message, action_error)}

                        {render_tabs(inventory.tabs.clone(), tab_key.clone(), set_selected_tab)}
                        <ResourceTable
                            resources=resources.clone()
                            active_tab=active_tab
                            on_view_resource=on_view_resource
                            row_actions=row_actions
                        />
                    </Panel>

                    <div class="service-side-stack">
                        <JsonInspectorPanel title="JSON inspector" json=inventory_json icon=LuFileJson />
                    </div>
                    {move || render_action_dialog(
                        create_dialog_service_key.clone(),
                        domain,
                        active_action.get(),
                        set_active_action,
                        action_message,
                        set_action_message,
                        action_error,
                        set_action_error,
                        action_loading,
                        set_action_loading,
                        primary_ref,
                        secondary_ref,
                        payload_ref,
                        set_inventory,
                        set_selected_resource,
                    )}
                    {move || render_resource_action_dialog(
                        resource_dialog_service_key.clone(),
                        resource_action.get(),
                        set_resource_action,
                        action_message,
                        set_action_message,
                        action_error,
                        set_action_error,
                        action_loading,
                        set_action_loading,
                        primary_ref,
                        secondary_ref,
                        payload_ref,
                        set_inventory,
                        set_selected_resource,
                    )}
                    {move || render_delete_dialog(
                        delete_dialog_service_key.clone(),
                        delete_action.get(),
                        set_delete_action,
                        action_error,
                        set_action_message,
                        set_action_error,
                        action_loading,
                        set_action_loading,
                        set_inventory,
                        set_selected_resource,
                    )}
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

fn render_action_toolbar(
    service_key: String,
    actions: &'static [ServiceActionDefinition],
    set_active_action: WriteSignal<Option<ServiceActionDefinition>>,
    set_action_message: WriteSignal<Option<String>>,
    set_action_error: WriteSignal<Option<String>>,
) -> View {
    let toolbar_actions = actions
        .iter()
        .copied()
        .filter(|action| {
            let is_service_unsupported =
                matches!(action.kind, ActionKind::Unsupported) && !action.requires_selection;
            matches!(action.kind, ActionKind::Create) || is_service_unsupported
        })
        .collect::<Vec<_>>();

    if toolbar_actions.is_empty() {
        return view! { <></> }.into_view();
    }

    view! {
        <div class="safe-action-toolbar">
            {toolbar_actions.into_iter().map(|action| {
                let action_service_key = service_key.clone();
                let disabled = matches!(action.kind, ActionKind::Unsupported);
                view! {
                    <button
                        type="button"
                        class=action_button_class(action.kind)
                        title=format!("Run {} for {}", action.label, action_service_key)
                        disabled=disabled
                        on:click=move |_| {
                            if disabled {
                                set_action_message.set(None);
                                set_action_error.set(Some(
                                    "Unsupported until Floci reports local API support.".to_owned(),
                                ));
                            } else {
                                set_action_error.set(None);
                                set_action_message.set(None);
                                set_active_action.set(Some(action));
                            }
                        }
                    >
                        <Icon
                            icon=if disabled { LuCircleSlash } else { LuPlus }
                            width="1em"
                            height="1em"
                        />
                        {action.label}
                    </button>
                }
                .into_view()
            }).collect_view()}
        </div>
    }
    .into_view()
}

#[allow(clippy::too_many_arguments)]
fn render_action_dialog(
    service_key: String,
    domain: &'static ServiceDomainDefinition,
    active_action: Option<ServiceActionDefinition>,
    set_active_action: WriteSignal<Option<ServiceActionDefinition>>,
    action_message: ReadSignal<Option<String>>,
    set_action_message: WriteSignal<Option<String>>,
    action_error: ReadSignal<Option<String>>,
    set_action_error: WriteSignal<Option<String>>,
    action_loading: ReadSignal<bool>,
    set_action_loading: WriteSignal<bool>,
    primary_ref: NodeRef<Input>,
    secondary_ref: NodeRef<Input>,
    payload_ref: NodeRef<Textarea>,
    set_inventory: WriteSignal<RemoteData<ServiceInventory>>,
    set_selected_resource: WriteSignal<Option<ResourceSummary>>,
) -> View {
    let Some(action) = active_action else {
        return view! { <></> }.into_view();
    };

    if !matches!(action.kind, ActionKind::Create) {
        return view! { <></> }.into_view();
    }

    let title = action.label;
    let footer = view! {
        <div class="dialog-actions">
            <Button
                variant=ButtonVariant::Ghost
                size=ButtonSize::Medium
                disabled=action_loading
                on_click=Callback::new(move |_| {
                    set_action_error.set(None);
                    set_active_action.set(None);
                })
            >
                "Cancel"
            </Button>
            <Button
                variant=ButtonVariant::Primary
                size=ButtonSize::Medium
                loading=action_loading
                on_click=Callback::new(move |_| {
                    let primary = input_value(primary_ref);
                    let secondary = input_value(secondary_ref);
                    let payload = textarea_value(payload_ref);
                    dispatch_action(
                        service_key.clone(),
                        action,
                        None,
                        primary,
                        secondary,
                        payload,
                        String::new(),
                        set_action_message,
                        set_action_error,
                        set_action_loading,
                        set_inventory,
                        set_selected_resource,
                        Callback::new(move |_| set_active_action.set(None)),
                    );
                })
            >
                "Create"
            </Button>
        </div>
    }
    .into_view();

    view! {
        <Dialog
            title=title
            eyebrow="Create resource"
            description=action_description(action, String::new())
            open=true
            loading=action_loading
            footer=footer
            on_close=Callback::new(move |_| {
                if !action_loading.get_untracked() {
                    set_action_error.set(None);
                    set_active_action.set(None);
                }
            })
        >
            <div class="action-form modal">
                <label>
                    <span>{domain.create_field_label.unwrap_or("Name")}</span>
                    <input
                        node_ref=primary_ref
                        type="text"
                        placeholder=domain.create_placeholder.unwrap_or("resource-name")
                    />
                </label>
                {domain.secondary_field_label.map(|label| view! {
                    <label>
                        <span>{label}</span>
                        <input
                            node_ref=secondary_ref
                            type="text"
                            placeholder=domain.secondary_placeholder.unwrap_or("id")
                        />
                    </label>
                })}
                {create_payload_label(action).map(|label| view! {
                    <label>
                        <span>{label}</span>
                        <textarea
                            node_ref=payload_ref
                            rows="7"
                            placeholder=create_payload_placeholder(action)
                        ></textarea>
                    </label>
                })}
                <JsonInspectorPanel
                    title="Payload preview"
                    json=create_action_preview_json(action)
                    icon=LuFileJson
                    description="Review the request shape before sending it to the local emulator."
                />
                {render_action_feedback(action_message, action_error)}
            </div>
        </Dialog>
    }
    .into_view()
}

#[allow(clippy::too_many_arguments)]
fn resource_row_actions(
    service_key: String,
    actions: &'static [ServiceActionDefinition],
    unsupported_operations: Vec<String>,
    set_selected_resource: WriteSignal<Option<ResourceSummary>>,
    set_resource_action: WriteSignal<Option<(ServiceActionDefinition, ResourceSummary)>>,
    set_delete_action: WriteSignal<Option<(ServiceActionDefinition, ResourceSummary)>>,
    set_action_message: WriteSignal<Option<String>>,
    set_action_error: WriteSignal<Option<String>>,
    set_action_loading: WriteSignal<bool>,
    set_inventory: WriteSignal<RemoteData<ServiceInventory>>,
) -> Vec<ResourceRowAction> {
    actions
        .iter()
        .copied()
        .filter(|action| !matches!(action.kind, ActionKind::Create) && action.requires_selection)
        .map(|action| {
            let unsupported_for_disabled = unsupported_operations.clone();
            let unsupported_for_title = unsupported_operations.clone();
            let action_service_key = service_key.clone();
            let tone = match action.kind {
                ActionKind::Delete => ResourceRowActionTone::Danger,
                ActionKind::Unsupported => ResourceRowActionTone::Muted,
                ActionKind::Create | ActionKind::Refresh | ActionKind::Execute => {
                    ResourceRowActionTone::Neutral
                }
            };
            let on_click = match action.kind {
                ActionKind::Refresh => Callback::new(move |resource: ResourceSummary| {
                    set_selected_resource.set(Some(resource.clone()));
                    dispatch_action(
                        action_service_key.clone(),
                        action,
                        Some(resource),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        set_action_message,
                        set_action_error,
                        set_action_loading,
                        set_inventory,
                        set_selected_resource,
                        Callback::new(|_: ()| {}),
                    );
                }),
                ActionKind::Execute => Callback::new(move |resource: ResourceSummary| {
                    set_action_message.set(None);
                    set_action_error.set(None);
                    set_selected_resource.set(Some(resource.clone()));
                    set_resource_action.set(Some((action, resource)));
                }),
                ActionKind::Delete => Callback::new(move |resource: ResourceSummary| {
                    set_action_message.set(None);
                    set_action_error.set(None);
                    set_selected_resource.set(Some(resource.clone()));
                    set_delete_action.set(Some((action, resource)));
                }),
                ActionKind::Create | ActionKind::Unsupported => {
                    Callback::new(|_: ResourceSummary| {})
                }
            };

            ResourceRowAction {
                label: action.label.to_owned(),
                icon: action_icon(action.kind),
                visible: Rc::new(move |resource| {
                    action
                        .resource_kind
                        .is_none_or(|kind| kind == resource.kind.as_str())
                }),
                title: Rc::new(move |resource| {
                    action_disabled_reason(action, &unsupported_for_title, resource)
                        .unwrap_or_else(|| format!("Run {}", action.label))
                }),
                disabled: Rc::new(move |resource| {
                    action_disabled_reason(action, &unsupported_for_disabled, resource).is_some()
                }),
                tone,
                on_click,
            }
        })
        .collect()
}

fn action_icon(kind: ActionKind) -> IconData {
    match kind {
        ActionKind::Refresh => LuRefreshCw,
        ActionKind::Execute => LuPackageSearch,
        ActionKind::Delete => LuTrash2,
        ActionKind::Create | ActionKind::Unsupported => LuCircleSlash,
    }
}

#[allow(clippy::too_many_arguments)]
fn render_resource_action_dialog(
    service_key: String,
    pending_action: Option<(ServiceActionDefinition, ResourceSummary)>,
    set_resource_action: WriteSignal<Option<(ServiceActionDefinition, ResourceSummary)>>,
    action_message: ReadSignal<Option<String>>,
    set_action_message: WriteSignal<Option<String>>,
    action_error: ReadSignal<Option<String>>,
    set_action_error: WriteSignal<Option<String>>,
    action_loading: ReadSignal<bool>,
    set_action_loading: WriteSignal<bool>,
    primary_ref: NodeRef<Input>,
    secondary_ref: NodeRef<Input>,
    payload_ref: NodeRef<Textarea>,
    set_inventory: WriteSignal<RemoteData<ServiceInventory>>,
    set_selected_resource: WriteSignal<Option<ResourceSummary>>,
) -> View {
    let Some((action, resource)) = pending_action else {
        return view! { <></> }.into_view();
    };

    if !matches!(action.kind, ActionKind::Execute) {
        return view! { <></> }.into_view();
    }

    let config = action_form_config(action);
    let resource_name = resource.name.clone();
    let selected_resource = resource.clone();
    let footer_resource = resource.clone();
    let footer = view! {
        <div class="dialog-actions">
            <Button
                variant=ButtonVariant::Ghost
                size=ButtonSize::Medium
                disabled=action_loading
                on_click=Callback::new(move |_| {
                    set_action_error.set(None);
                    set_resource_action.set(None);
                })
            >
                "Cancel"
            </Button>
            <Button
                variant=ButtonVariant::Primary
                size=ButtonSize::Medium
                loading=action_loading
                on_click=Callback::new(move |_| {
                    let primary = input_value(primary_ref);
                    let secondary = input_value(secondary_ref);
                    let payload = textarea_value(payload_ref);
                    dispatch_action(
                        service_key.clone(),
                        action,
                        Some(footer_resource.clone()),
                        primary,
                        secondary,
                        payload,
                        String::new(),
                        set_action_message,
                        set_action_error,
                        set_action_loading,
                        set_inventory,
                        set_selected_resource,
                        Callback::new(move |_| set_resource_action.set(None)),
                    );
                })
            >
                {config.submit_label}
            </Button>
        </div>
    }
    .into_view();

    view! {
        <Dialog
            title=action.label
            eyebrow=format!("{} · {}", resource.kind, resource_name)
            description=action_description(action, resource.name.clone())
            open=true
            loading=action_loading
            footer=footer
            on_close=Callback::new(move |_| {
                if !action_loading.get_untracked() {
                    set_action_error.set(None);
                    set_resource_action.set(None);
                }
            })
        >
            <div class="action-form modal">
                {is_reveal_action(action).then(|| view! {
                    <InlineNotice tone=NoticeTone::Warning title="Sensitive value reveal">
                        "This asks the local emulator for the value only after this click. The result is summarized and cleared when the action dialog closes; inventory and screenshots stay redacted."
                    </InlineNotice>
                })}
                {config.primary_label.map(|label| view! {
                    <label>
                        <span>{label}</span>
                        <input
                            node_ref=primary_ref
                            type="text"
                            placeholder=config.primary_placeholder
                        />
                    </label>
                })}
                {config.secondary_label.map(|label| view! {
                    <label>
                        <span>{label}</span>
                        <input
                            node_ref=secondary_ref
                            type="text"
                            placeholder=config.secondary_placeholder
                        />
                    </label>
                })}
                {config.payload_label.map(|label| view! {
                    <label>
                        <span>{label}</span>
                        <textarea
                            node_ref=payload_ref
                            rows="7"
                            placeholder=config.payload_placeholder
                        ></textarea>
                    </label>
                })}
                <JsonInspectorPanel
                    title="Payload preview"
                    json=action_preview_json(action, &selected_resource)
                    icon=LuFileJson
                    description="Review the request shape before sending it to the selected local resource."
                />
                {render_action_feedback(action_message, action_error)}
            </div>
        </Dialog>
    }
    .into_view()
}

#[allow(clippy::too_many_arguments)]
fn render_delete_dialog(
    service_key: String,
    pending_action: Option<(ServiceActionDefinition, ResourceSummary)>,
    set_delete_action: WriteSignal<Option<(ServiceActionDefinition, ResourceSummary)>>,
    action_error: ReadSignal<Option<String>>,
    set_action_message: WriteSignal<Option<String>>,
    set_action_error: WriteSignal<Option<String>>,
    action_loading: ReadSignal<bool>,
    set_action_loading: WriteSignal<bool>,
    set_inventory: WriteSignal<RemoteData<ServiceInventory>>,
    set_selected_resource: WriteSignal<Option<ResourceSummary>>,
) -> View {
    let Some((action, resource)) = pending_action else {
        return view! { <></> }.into_view();
    };

    let resource_name = resource.name.clone();
    let confirmation_name = resource_name.clone();
    let selected_resource = resource.clone();
    let dialog_title = destructive_title(action, &resource_name);
    let dialog_message = destructive_message(action);
    let confirm_label = destructive_confirm_label(action);
    view! {
        <ConfirmDialog
            title=dialog_title
            message=dialog_message
            confirm_label=confirm_label
            open=true
            tone=NoticeTone::Danger
            loading=action_loading
            required_confirmation=confirmation_name
            error=action_error.get().unwrap_or_default()
            on_cancel=Callback::new(move |_| {
                if !action_loading.get_untracked() {
                    set_action_error.set(None);
                    set_delete_action.set(None);
                }
            })
            on_confirm=Callback::new(move |confirmation| {
                dispatch_action(
                    service_key.clone(),
                    action,
                    Some(selected_resource.clone()),
                    String::new(),
                    String::new(),
                    String::new(),
                    confirmation,
                    set_action_message,
                    set_action_error,
                    set_action_loading,
                    set_inventory,
                    set_selected_resource,
                    Callback::new(move |_| set_delete_action.set(None)),
                );
            })
        />
    }
    .into_view()
}

fn action_disabled_reason(
    action: ServiceActionDefinition,
    unsupported_operations: &[String],
    resource: &ResourceSummary,
) -> Option<String> {
    if unsupported_operations
        .iter()
        .any(|operation| operation == action.key)
        || matches!(action.kind, ActionKind::Unsupported)
    {
        return Some("Unsupported until Floci reports local API support.".to_owned());
    }

    if let Some(kind) = action.resource_kind {
        if resource.kind != kind {
            return Some(format!(
                "Select a {kind} resource before running this action."
            ));
        }
    }

    None
}

#[allow(clippy::too_many_arguments)]
fn dispatch_action(
    service_key: String,
    action: ServiceActionDefinition,
    selected_resource: Option<ResourceSummary>,
    primary: String,
    secondary: String,
    payload_text: String,
    confirmation: String,
    set_action_message: WriteSignal<Option<String>>,
    set_action_error: WriteSignal<Option<String>>,
    set_action_loading: WriteSignal<bool>,
    set_inventory: WriteSignal<RemoteData<ServiceInventory>>,
    set_selected_resource: WriteSignal<Option<ResourceSummary>>,
    on_success: Callback<()>,
) {
    if matches!(action.kind, ActionKind::Create) && primary.trim().is_empty() {
        set_action_error.set(Some("A resource name is required.".to_owned()));
        return;
    }

    if let Some(message) = validate_action_inputs(action, &primary, &secondary, &payload_text) {
        set_action_error.set(Some(message));
        return;
    }

    if action.requires_selection && selected_resource.is_none() {
        set_action_error.set(Some(
            "Select a resource before running this action.".to_owned(),
        ));
        return;
    }
    if let (Some(kind), Some(resource)) = (action.resource_kind, selected_resource.as_ref()) {
        if resource.kind != kind {
            set_action_error.set(Some(format!(
                "Select a `{kind}` resource before running this action."
            )));
            return;
        }
    }

    let payload = action_payload(
        service_key.as_str(),
        action,
        primary,
        secondary,
        payload_text,
    );
    let request = ServiceActionRequest {
        service_key: service_key.clone(),
        action: action.key.to_owned(),
        resource_id: selected_resource
            .as_ref()
            .map(|resource| resource.id.clone()),
        resource_name: selected_resource
            .as_ref()
            .map(|resource| resource.name.clone()),
        confirmation: (!confirmation.trim().is_empty()).then(|| confirmation.trim().to_owned()),
        payload,
    };

    set_action_loading.set(true);
    set_action_error.set(None);
    set_action_message.set(None);

    spawn_local(async move {
        match client::service_execute_action(request).await {
            Ok(ActionResult { message, .. }) => {
                set_action_message.set(Some(message));
                set_inventory.set(RemoteData::Loading);
                set_selected_resource.set(None);
                set_inventory.set(remote_result(client::service_inventory(service_key).await));
                on_success.call(());
            }
            Err(message) => {
                set_action_error.set(Some(message));
            }
        }

        set_action_loading.set(false);
    });
}

fn action_payload(
    service_key: &str,
    action: ServiceActionDefinition,
    primary: String,
    secondary: String,
    payload_text: String,
) -> Value {
    match (service_key, action.key) {
        ("s3", "create_bucket") => json!({ "bucket_name": primary.trim() }),
        ("dynamodb", "create_table") => {
            json!({
                "table_name": primary.trim(),
                "partition_key": if secondary.trim().is_empty() {
                    "id"
                } else {
                    secondary.trim()
                },
            })
        }
        ("sqs", "create_queue") => json!({ "queue_name": primary.trim() }),
        ("sqs", "send_message") => json!({ "message_body": payload_text.trim() }),
        ("sns", "create_topic") => json!({ "topic_name": primary.trim() }),
        ("sns", "publish_message") => json!({ "message": payload_text.trim() }),
        ("sns", "subscribe_endpoint") => {
            json!({
                "protocol": primary.trim(),
                "endpoint": secondary.trim(),
            })
        }
        ("kinesis", "create_stream") => json!({ "stream_name": primary.trim() }),
        ("kinesis", "put_record") => {
            json!({
                "partition_key": primary.trim(),
                "data": payload_text.trim(),
            })
        }
        ("eventbridge", "create_event_bus") => json!({ "event_bus_name": primary.trim() }),
        ("eventbridge", "create_rule") => {
            json!({
                "rule_name": primary.trim(),
                "event_pattern": payload_text.trim(),
            })
        }
        ("eventbridge", "put_event") => {
            json!({
                "source": primary.trim(),
                "detail_type": secondary.trim(),
                "detail": payload_text.trim(),
            })
        }
        ("stepfunctions", "create_state_machine") => {
            json!({
                "state_machine_name": primary.trim(),
                "role_arn": secondary.trim(),
                "definition": payload_text.trim(),
            })
        }
        ("stepfunctions", "start_execution") => {
            json!({
                "execution_name": primary.trim(),
                "input": payload_text.trim(),
            })
        }
        ("cloudformation", "create_stack") => {
            json!({
                "stack_name": primary.trim(),
                "template_body": payload_text.trim(),
            })
        }
        ("lambda", "create_function") => {
            json!({
                "function_name": primary.trim(),
                "role_arn": secondary.trim(),
            })
        }
        ("lambda", "invoke_function") => json!({ "payload": payload_text.trim() }),
        ("ecs", "run_task") => json!({ "task_definition": primary.trim() }),
        ("ecs", "update_service_desired_count") => {
            json!({
                "desired_count": primary.trim(),
                "cluster_arn": secondary.trim(),
            })
        }
        ("ecr", "create_repository") => json!({ "repository_name": primary.trim() }),
        ("codebuild", "start_build") => json!({ "source_version": primary.trim() }),
        ("codedeploy", "create_deployment") => {
            json!({
                "description": payload_text.trim(),
            })
        }
        ("autoscaling", "update_desired_capacity") => {
            json!({ "desired_capacity": primary.trim() })
        }
        ("bedrockruntime", "invoke_model") => {
            json!({
                "model_id": primary.trim(),
                "body": payload_text.trim(),
            })
        }
        ("bedrockruntime", "validate_request_template") => json!({ "body": payload_text.trim() }),
        ("iam", "create_user") => json!({ "user_name": primary.trim() }),
        ("iam", "create_role") => {
            json!({
                "role_name": primary.trim(),
                "assume_role_policy_document": payload_text.trim(),
            })
        }
        ("iam", "create_policy") => {
            json!({
                "policy_name": primary.trim(),
                "policy_document": payload_text.trim(),
            })
        }
        ("iam", "create_access_key") => json!({}),
        ("cognito", "create_user_pool") => json!({ "pool_name": primary.trim() }),
        ("cognito", "create_user_pool_client") => json!({ "client_name": primary.trim() }),
        ("kms", "create_key") => json!({ "description": primary.trim() }),
        ("kms", "create_alias") => json!({ "alias_name": primary.trim() }),
        ("secretsmanager", "reveal_secret_value")
        | ("secretsmanager", "rotate_secret")
        | ("ssm", "reveal_parameter_value") => json!({}),
        ("appconfig", "create_application") => json!({ "application_name": primary.trim() }),
        ("appconfig", "create_environment") => json!({ "environment_name": primary.trim() }),
        ("appconfig", "create_configuration_profile") => {
            json!({
                "profile_name": primary.trim(),
                "location_uri": secondary.trim(),
            })
        }
        ("appconfig", "create_hosted_version") => {
            json!({
                "content": payload_text.trim(),
                "content_type": secondary.trim(),
            })
        }
        ("appconfig", "start_deployment") => {
            json!({
                "environment_id": primary.trim(),
                "deployment_strategy_id": secondary.trim(),
            })
        }
        _ => json!({}),
    }
}

#[derive(Clone, Copy)]
struct ActionFormConfig {
    primary_label: Option<&'static str>,
    primary_placeholder: &'static str,
    secondary_label: Option<&'static str>,
    secondary_placeholder: &'static str,
    payload_label: Option<&'static str>,
    payload_placeholder: &'static str,
    submit_label: &'static str,
}

fn action_form_config(action: ServiceActionDefinition) -> ActionFormConfig {
    match action.key {
        "subscribe_endpoint" => ActionFormConfig {
            primary_label: Some("Protocol"),
            primary_placeholder: "sqs",
            secondary_label: Some("Endpoint"),
            secondary_placeholder: "arn:aws:sqs:us-east-1:000000000000:orders-events",
            payload_label: None,
            payload_placeholder: "",
            submit_label: "Subscribe",
        },
        "put_record" => ActionFormConfig {
            primary_label: Some("Partition key"),
            primary_placeholder: "order-123",
            secondary_label: None,
            secondary_placeholder: "",
            payload_label: Some("Record data"),
            payload_placeholder: r#"{"event":"order.created"}"#,
            submit_label: "Put record",
        },
        "create_rule" => ActionFormConfig {
            primary_label: Some("Rule name"),
            primary_placeholder: "order-created",
            secondary_label: None,
            secondary_placeholder: "",
            payload_label: Some("Event pattern JSON"),
            payload_placeholder: r#"{"source":["floci.ui"]}"#,
            submit_label: "Create rule",
        },
        "put_event" => ActionFormConfig {
            primary_label: Some("Source"),
            primary_placeholder: "floci.ui",
            secondary_label: Some("Detail type"),
            secondary_placeholder: "ManualTest",
            payload_label: Some("Detail JSON"),
            payload_placeholder: r#"{"status":"ok"}"#,
            submit_label: "Put event",
        },
        "start_execution" => ActionFormConfig {
            primary_label: Some("Execution name"),
            primary_placeholder: "manual-test",
            secondary_label: None,
            secondary_placeholder: "",
            payload_label: Some("Input JSON"),
            payload_placeholder: "{}",
            submit_label: "Start execution",
        },
        "invoke_function" => ActionFormConfig {
            primary_label: None,
            primary_placeholder: "",
            secondary_label: None,
            secondary_placeholder: "",
            payload_label: Some("Payload JSON"),
            payload_placeholder: r#"{"message":"hello from floci-ui"}"#,
            submit_label: "Invoke",
        },
        "run_task" => ActionFormConfig {
            primary_label: Some("Task definition"),
            primary_placeholder: "orders-worker:1",
            secondary_label: None,
            secondary_placeholder: "",
            payload_label: None,
            payload_placeholder: "",
            submit_label: "Run task",
        },
        "update_service_desired_count" => ActionFormConfig {
            primary_label: Some("Desired count"),
            primary_placeholder: "2",
            secondary_label: Some("Cluster ARN"),
            secondary_placeholder: "arn:aws:ecs:us-east-1:000000000000:cluster/default",
            payload_label: None,
            payload_placeholder: "",
            submit_label: "Update service",
        },
        "start_build" => ActionFormConfig {
            primary_label: Some("Source version"),
            primary_placeholder: "main",
            secondary_label: None,
            secondary_placeholder: "",
            payload_label: None,
            payload_placeholder: "",
            submit_label: "Start build",
        },
        "create_deployment" => ActionFormConfig {
            primary_label: None,
            primary_placeholder: "",
            secondary_label: None,
            secondary_placeholder: "",
            payload_label: Some("Deployment description"),
            payload_placeholder: "floci-ui local deployment",
            submit_label: "Create deployment",
        },
        "update_desired_capacity" => ActionFormConfig {
            primary_label: Some("Desired capacity"),
            primary_placeholder: "2",
            secondary_label: None,
            secondary_placeholder: "",
            payload_label: None,
            payload_placeholder: "",
            submit_label: "Update capacity",
        },
        "invoke_model" => ActionFormConfig {
            primary_label: Some("Model ID override"),
            primary_placeholder: "local-bedrock-runtime",
            secondary_label: None,
            secondary_placeholder: "",
            payload_label: Some("Request JSON"),
            payload_placeholder: r#"{"prompt":"hello from floci-ui"}"#,
            submit_label: "Invoke model",
        },
        "validate_request_template" => ActionFormConfig {
            primary_label: None,
            primary_placeholder: "",
            secondary_label: None,
            secondary_placeholder: "",
            payload_label: Some("Request JSON"),
            payload_placeholder: r#"{"prompt":"hello from floci-ui"}"#,
            submit_label: "Validate",
        },
        "send_message" => ActionFormConfig {
            primary_label: None,
            primary_placeholder: "",
            secondary_label: None,
            secondary_placeholder: "",
            payload_label: Some("Message body"),
            payload_placeholder: r#"{"message":"hello from floci-ui"}"#,
            submit_label: "Send message",
        },
        "publish_message" => ActionFormConfig {
            primary_label: None,
            primary_placeholder: "",
            secondary_label: None,
            secondary_placeholder: "",
            payload_label: Some("Message"),
            payload_placeholder: r#"{"message":"hello subscribers"}"#,
            submit_label: "Publish",
        },
        "create_access_key" => ActionFormConfig {
            primary_label: None,
            primary_placeholder: "",
            secondary_label: None,
            secondary_placeholder: "",
            payload_label: None,
            payload_placeholder: "",
            submit_label: "Create key",
        },
        "create_user_pool_client" => ActionFormConfig {
            primary_label: Some("Client name"),
            primary_placeholder: "floci-local-client",
            secondary_label: None,
            secondary_placeholder: "",
            payload_label: None,
            payload_placeholder: "",
            submit_label: "Create client",
        },
        "create_alias" => ActionFormConfig {
            primary_label: Some("Alias name"),
            primary_placeholder: "alias/floci-local",
            secondary_label: None,
            secondary_placeholder: "",
            payload_label: None,
            payload_placeholder: "",
            submit_label: "Create alias",
        },
        "reveal_secret_value" | "reveal_parameter_value" => ActionFormConfig {
            primary_label: None,
            primary_placeholder: "",
            secondary_label: None,
            secondary_placeholder: "",
            payload_label: None,
            payload_placeholder: "",
            submit_label: "Reveal summary",
        },
        "create_environment" => ActionFormConfig {
            primary_label: Some("Environment name"),
            primary_placeholder: "local",
            secondary_label: None,
            secondary_placeholder: "",
            payload_label: None,
            payload_placeholder: "",
            submit_label: "Create environment",
        },
        "create_configuration_profile" => ActionFormConfig {
            primary_label: Some("Profile name"),
            primary_placeholder: "runtime-config",
            secondary_label: Some("Location URI"),
            secondary_placeholder: "hosted",
            payload_label: None,
            payload_placeholder: "",
            submit_label: "Create profile",
        },
        "create_hosted_version" => ActionFormConfig {
            primary_label: None,
            primary_placeholder: "",
            secondary_label: Some("Content type"),
            secondary_placeholder: "application/json",
            payload_label: Some("Configuration JSON"),
            payload_placeholder: "{}",
            submit_label: "Create version",
        },
        "start_deployment" => ActionFormConfig {
            primary_label: Some("Environment ID"),
            primary_placeholder: "local",
            secondary_label: Some("Deployment strategy ID"),
            secondary_placeholder: "AppConfig.AllAtOnce",
            payload_label: None,
            payload_placeholder: "",
            submit_label: "Start deployment",
        },
        _ => ActionFormConfig {
            primary_label: None,
            primary_placeholder: "",
            secondary_label: None,
            secondary_placeholder: "",
            payload_label: None,
            payload_placeholder: "",
            submit_label: "Run",
        },
    }
}

fn validate_action_inputs(
    action: ServiceActionDefinition,
    primary: &str,
    secondary: &str,
    payload_text: &str,
) -> Option<String> {
    match action.key {
        "send_message" | "publish_message" => payload_text
            .trim()
            .is_empty()
            .then(|| "A message body is required before this local action can run.".to_owned()),
        "put_record" => {
            if primary.trim().is_empty() {
                return Some("A partition key is required.".to_owned());
            }
            payload_text
                .trim()
                .is_empty()
                .then(|| "Record data is required.".to_owned())
        }
        "subscribe_endpoint" => {
            if primary.trim().is_empty() {
                return Some("A subscription protocol is required.".to_owned());
            }
            secondary
                .trim()
                .is_empty()
                .then(|| "A subscription endpoint is required.".to_owned())
        }
        "create_rule" => {
            if primary.trim().is_empty() {
                return Some("A rule name is required.".to_owned());
            }
            if payload_text.trim().is_empty() {
                return Some("An event pattern JSON payload is required.".to_owned());
            }
            validate_json_payload(payload_text, "Event pattern JSON")
        }
        "put_event" => {
            if primary.trim().is_empty() {
                return Some("An event source is required.".to_owned());
            }
            if secondary.trim().is_empty() {
                return Some("A detail type is required.".to_owned());
            }
            if payload_text.trim().is_empty() {
                return Some("A detail JSON payload is required.".to_owned());
            }
            validate_json_payload(payload_text, "Detail JSON")
        }
        "create_state_machine" => {
            if payload_text.trim().is_empty() {
                return None;
            }
            validate_json_payload(payload_text, "Definition JSON")
        }
        "start_execution" => {
            if payload_text.trim().is_empty() {
                return None;
            }
            validate_json_payload(payload_text, "Input JSON")
        }
        "invoke_function" => {
            if payload_text.trim().is_empty() {
                return Some("A JSON payload is required.".to_owned());
            }
            validate_json_payload(payload_text, "Payload JSON")
        }
        "run_task" => primary
            .trim()
            .is_empty()
            .then(|| "A task definition is required.".to_owned()),
        "update_service_desired_count" => {
            if primary.trim().parse::<i32>().is_err() {
                return Some("Desired count must be a number.".to_owned());
            }
            secondary
                .trim()
                .is_empty()
                .then(|| "A cluster ARN is required.".to_owned())
        }
        "update_desired_capacity" => primary
            .trim()
            .parse::<i32>()
            .is_err()
            .then(|| "Desired capacity must be a number.".to_owned()),
        "invoke_model" | "validate_request_template" => {
            if payload_text.trim().is_empty() {
                return Some("A request JSON payload is required.".to_owned());
            }
            validate_json_payload(payload_text, "Request JSON")
        }
        "create_stack" => {
            if payload_text.trim().is_empty() {
                return None;
            }
            validate_json_payload(payload_text, "Template body JSON")
        }
        "create_role" => {
            if payload_text.trim().is_empty() {
                return None;
            }
            validate_json_payload(payload_text, "Assume role policy JSON")
        }
        "create_policy" => {
            if payload_text.trim().is_empty() {
                return None;
            }
            validate_json_payload(payload_text, "Policy document JSON")
        }
        "create_alias" => primary
            .trim()
            .starts_with("alias/")
            .then_some(())
            .is_none()
            .then(|| "Alias name must start with `alias/`.".to_owned()),
        "create_user_pool_client" | "create_environment" | "create_configuration_profile" => {
            primary
                .trim()
                .is_empty()
                .then(|| "A name is required.".to_owned())
        }
        "create_hosted_version" => {
            if payload_text.trim().is_empty() {
                return Some("Configuration JSON is required.".to_owned());
            }
            validate_json_payload(payload_text, "Configuration JSON")
        }
        "start_deployment" => primary
            .trim()
            .is_empty()
            .then(|| "An environment ID is required.".to_owned()),
        _ => None,
    }
}

fn validate_json_payload(payload_text: &str, label: &str) -> Option<String> {
    serde_json::from_str::<Value>(payload_text.trim())
        .err()
        .map(|err| format!("{label} is not valid JSON: {err}"))
}

fn create_payload_label(action: ServiceActionDefinition) -> Option<&'static str> {
    match action.key {
        "create_state_machine" => Some("Definition JSON"),
        "create_stack" => Some("Template body JSON"),
        "create_role" => Some("Assume role policy JSON"),
        "create_policy" => Some("Policy document JSON"),
        _ => None,
    }
}

fn create_payload_placeholder(action: ServiceActionDefinition) -> &'static str {
    match action.key {
        "create_state_machine" => {
            r#"{"StartAt":"Pass","States":{"Pass":{"Type":"Pass","End":true}}}"#
        }
        "create_stack" => r#"{"AWSTemplateFormatVersion":"2010-09-09","Resources":{}}"#,
        "create_role" => {
            r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"Service":"lambda.amazonaws.com"},"Action":"sts:AssumeRole"}]}"#
        }
        "create_policy" => {
            r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Action":"*","Resource":"*"}]}"#
        }
        _ => "",
    }
}

fn action_preview_json(action: ServiceActionDefinition, resource: &ResourceSummary) -> String {
    serde_json::to_string_pretty(&json!({
        "service_action": action.key,
        "selected_resource": {
            "id": &resource.id,
            "name": &resource.name,
            "kind": &resource.kind,
        },
        "payload": "Form values are previewed before being sent to the local emulator."
    }))
    .unwrap_or_else(|_| "{}".to_owned())
}

fn create_action_preview_json(action: ServiceActionDefinition) -> String {
    serde_json::to_string_pretty(&json!({
        "service_action": action.key,
        "selected_resource": null,
        "payload": "Form values are validated before being sent to the local emulator."
    }))
    .unwrap_or_else(|_| "{}".to_owned())
}

fn destructive_title(action: ServiceActionDefinition, resource_name: &str) -> String {
    match action.key {
        "purge_queue" => format!("Purge {resource_name}"),
        "delete_subscription" => format!("Unsubscribe {resource_name}"),
        "stop_execution" => format!("Stop {resource_name}"),
        "stop_instance" => format!("Stop {resource_name}"),
        "terminate_instance" => format!("Terminate {resource_name}"),
        "stop_task" => format!("Stop {resource_name}"),
        "disable_key" => format!("Disable {resource_name}"),
        "schedule_key_deletion" => format!("Schedule deletion for {resource_name}"),
        "rotate_secret" => format!("Rotate {resource_name}"),
        _ => format!("Delete {resource_name}"),
    }
}

fn destructive_message(action: ServiceActionDefinition) -> &'static str {
    match action.key {
        "purge_queue" => {
            "This removes every visible and in-flight message from the selected local queue."
        }
        "delete_subscription" => "This removes the selected local subscription from its SNS topic.",
        "stop_execution" => "This stops the selected local Step Functions execution.",
        "stop_instance" => "This stops the selected local EC2 instance.",
        "terminate_instance" => {
            "This terminates the selected local EC2 instance and removes its running state."
        }
        "stop_task" => "This stops the selected local ECS task.",
        "delete_function" => "This deletes the selected local Lambda function metadata.",
        "delete_repository" => {
            "This deletes the selected local ECR repository and any image metadata in it."
        }
        "delete_image" => "This deletes the selected local ECR image metadata.",
        "delete_user" => "This deletes the selected local IAM user metadata.",
        "delete_role" => "This deletes the selected local IAM role metadata.",
        "delete_policy" => "This deletes the selected local IAM policy metadata.",
        "delete_access_key" => {
            "This deletes the selected local IAM access key. The secret value is not recoverable from the UI."
        }
        "delete_user_pool" => "This deletes the selected local Cognito user pool.",
        "delete_user_pool_client" => "This deletes the selected local Cognito app client.",
        "disable_key" => "This disables the selected local KMS key for future cryptographic use.",
        "schedule_key_deletion" => {
            "This schedules deletion for the selected local KMS key using the backend default waiting period."
        }
        "rotate_secret" => "This starts local rotation for the selected Secrets Manager secret.",
        _ => {
            "This removes the selected resource from the local emulator. This action cannot be undone from the UI."
        }
    }
}

fn destructive_confirm_label(action: ServiceActionDefinition) -> &'static str {
    match action.key {
        "purge_queue" => "Purge",
        "delete_subscription" => "Unsubscribe",
        "stop_execution" => "Stop",
        "stop_instance" | "stop_task" => "Stop",
        "terminate_instance" => "Terminate",
        "disable_key" => "Disable",
        "schedule_key_deletion" => "Schedule",
        "rotate_secret" => "Rotate",
        _ => "Delete",
    }
}

fn render_action_feedback(
    message: ReadSignal<Option<String>>,
    error: ReadSignal<Option<String>>,
) -> View {
    view! {
        <>
            {move || message.get().map(|message| view! {
                <InlineNotice tone=NoticeTone::Success title="Action complete">
                    {message}
                </InlineNotice>
            })}
            {move || error.get().map(|message| view! {
                <InlineNotice tone=NoticeTone::Danger title="Action blocked">
                    {message}
                </InlineNotice>
            })}
        </>
    }
    .into_view()
}

fn input_value(node_ref: NodeRef<Input>) -> String {
    node_ref
        .get()
        .map_or_else(String::new, |input| input.value())
}

fn textarea_value(node_ref: NodeRef<Textarea>) -> String {
    node_ref
        .get()
        .map_or_else(String::new, |textarea| textarea.value())
}

fn action_description(action: ServiceActionDefinition, selected_name: String) -> String {
    if is_reveal_action(action) {
        return format!(
            "Explicitly fetch and immediately clear a sensitive local value for {selected_name}."
        );
    }

    match action.kind {
        ActionKind::Create => {
            "Required fields are validated before the Tauri command runs.".to_owned()
        }
        ActionKind::Refresh => selected_name
            .is_empty()
            .then(|| "Refresh the latest inventory metadata.".to_owned())
            .unwrap_or_else(|| format!("Refresh metadata for {selected_name}.")),
        ActionKind::Execute => {
            format!("Preview and run {} against {selected_name}.", action.label)
        }
        ActionKind::Delete => {
            format!(
                "Type {selected_name} to confirm {} against the disposable local emulator.",
                action.label
            )
        }
        ActionKind::Unsupported => {
            "This operation is shown for planning but is not executable.".to_owned()
        }
    }
}

fn is_reveal_action(action: ServiceActionDefinition) -> bool {
    matches!(action.key, "reveal_secret_value" | "reveal_parameter_value")
}

fn action_button_class(kind: ActionKind) -> &'static str {
    match kind {
        ActionKind::Create => "safe-action-button primary",
        ActionKind::Refresh | ActionKind::Execute => "safe-action-button",
        ActionKind::Delete => "safe-action-button danger",
        ActionKind::Unsupported => "safe-action-button disabled",
    }
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

fn support_tone(level: crate::service_management::models::ServiceSupportLevel) -> StatusTone {
    match level {
        crate::service_management::models::ServiceSupportLevel::Managed => StatusTone::Up,
        crate::service_management::models::ServiceSupportLevel::ReadOnly => StatusTone::Muted,
        crate::service_management::models::ServiceSupportLevel::Unsupported => StatusTone::Neutral,
    }
}

fn remote_result<T>(result: Result<T, String>) -> RemoteData<T> {
    match result {
        Ok(value) => RemoteData::Ready(value),
        Err(message) => RemoteData::Failed(message),
    }
}
