#![allow(clippy::must_use_candidate, clippy::wildcard_imports)]

use std::rc::Rc;

use icondata::{
    Icon as IconData, LuActivity, LuAlertTriangle, LuArchive, LuArrowUpDown, LuBox, LuCable,
    LuCalendar, LuCheckCircle2, LuChevronDown, LuChevronUp, LuCircleDot, LuCloudCog, LuDatabase,
    LuEye, LuFileJson, LuFolder, LuGlobe, LuHardDrive, LuKeyRound, LuLayoutDashboard, LuLink2,
    LuMapPin, LuPackage, LuPackageSearch, LuRefreshCw, LuServer, LuShieldCheck, LuTable2, LuTags,
    LuWifi,
};
use leptos::{ev::KeyboardEvent, ev::MouseEvent, *};
use leptos_icons::Icon;

use crate::{
    components::EmptyState,
    service_management::models::{ResourceSummary, ResourceTab},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResourceSortColumn {
    Name,
    Kind,
    Status,
    Updated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceRowActionTone {
    Neutral,
    Danger,
    Muted,
}

#[derive(Clone)]
pub struct ResourceRowAction {
    pub label: String,
    pub icon: IconData,
    pub visible: Rc<dyn Fn(&ResourceSummary) -> bool>,
    pub title: Rc<dyn Fn(&ResourceSummary) -> String>,
    pub disabled: Rc<dyn Fn(&ResourceSummary) -> bool>,
    pub tone: ResourceRowActionTone,
    pub on_click: Callback<ResourceSummary>,
}

#[component]
pub fn ResourceTable(
    resources: Vec<ResourceSummary>,
    active_tab: Option<ResourceTab>,
    on_view_resource: Callback<ResourceSummary>,
    #[prop(optional)] row_actions: Vec<ResourceRowAction>,
) -> impl IntoView {
    if resources.is_empty() {
        let message = active_tab.map_or_else(
            || "No resources are available for this service.".to_owned(),
            |tab| tab.empty_message,
        );

        return view! {
            <EmptyState title="No resources" message=message />
        }
        .into_view();
    }

    let (sort_column, set_sort_column) = create_signal(ResourceSortColumn::Name);
    let (ascending, set_ascending) = create_signal(true);
    let sorted_resources = move || {
        let mut sorted = resources.clone();
        let column = sort_column.get();

        sorted.sort_by(|left, right| {
            let ordering = match column {
                ResourceSortColumn::Name => left.name.cmp(&right.name),
                ResourceSortColumn::Kind => left.kind.cmp(&right.kind),
                ResourceSortColumn::Status => left.status.cmp(&right.status),
                ResourceSortColumn::Updated => left
                    .updated_at
                    .as_ref()
                    .or(left.created_at.as_ref())
                    .cmp(&right.updated_at.as_ref().or(right.created_at.as_ref())),
            };

            if ascending.get() {
                ordering
            } else {
                ordering.reverse()
            }
        });

        sorted
    };
    let change_sort = move |column: ResourceSortColumn| {
        if sort_column.get_untracked() == column {
            set_ascending.update(|ascending| *ascending = !*ascending);
        } else {
            set_sort_column.set(column);
            set_ascending.set(true);
        }
    };

    view! {
        <div class="resource-table-wrap">
            <table class="resource-table">
                <thead>
                    <tr>
                        <th>{sort_header("Name", LuPackage, ResourceSortColumn::Name, sort_column, ascending, change_sort)}</th>
                        <th>{sort_header("Kind", LuBox, ResourceSortColumn::Kind, sort_column, ascending, change_sort)}</th>
                        <th>{sort_header("Status", LuCircleDot, ResourceSortColumn::Status, sort_column, ascending, change_sort)}</th>
                        <th>{sort_header("Updated", LuCalendar, ResourceSortColumn::Updated, sort_column, ascending, change_sort)}</th>
                        <th>
                            <span class="table-header-label">
                                <Icon icon=LuTags width="1em" height="1em" />
                                "Attributes"
                            </span>
                        </th>
                        <th>"Actions"</th>
                    </tr>
                </thead>
                <tbody>
                    {move || sorted_resources().into_iter().map(|resource| {
                        let row_resource = resource.clone();
                        let keyboard_resource = resource.clone();
                        let view_resource = resource.clone();
                        let actions = row_actions.clone();
                        let kind_icon = resource_kind_icon(&resource.kind);
                        let status_icon = resource_status_icon(&resource.status);
                        let status_class = resource_status_class(&resource.status);
                        let updated_at = resource
                            .updated_at
                            .clone()
                            .or(resource.created_at.clone())
                            .unwrap_or_else(|| "n/a".to_owned());
                        let attribute_summary = attribute_summary_text(&resource);
                        view! {
                            <tr
                                tabindex="0"
                                on:click=move |_| on_view_resource.call(row_resource.clone())
                                on:keydown=move |event: KeyboardEvent| {
                                    if event.key() == "Enter" || event.key() == " " {
                                        event.prevent_default();
                                        on_view_resource.call(keyboard_resource.clone());
                                    }
                                }
                            >
                                <td>
                                    <div class="resource-name-cell">
                                        <span class="resource-kind-icon" aria-hidden="true">
                                            <Icon icon=kind_icon width="1em" height="1em" />
                                        </span>
                                        <div>
                                            <strong>{resource.name.clone()}</strong>
                                            <code>{resource.id.clone()}</code>
                                        </div>
                                    </div>
                                </td>
                                <td>
                                    <span class="resource-kind-inline">
                                        <Icon icon=kind_icon width="1em" height="1em" />
                                        {resource.kind.clone()}
                                    </span>
                                </td>
                                <td>
                                    <span class=status_class>
                                        <Icon icon=status_icon width="0.85em" height="0.85em" />
                                        {resource.status.clone()}
                                    </span>
                                </td>
                                <td>
                                    <span class="resource-date-cell">
                                        <Icon icon=LuCalendar width="1em" height="1em" />
                                        {updated_at}
                                    </span>
                                </td>
                                <td>
                                    <span class="resource-attribute-summary">
                                        <Icon icon=LuTags width="1em" height="1em" />
                                        {attribute_summary}
                                    </span>
                                </td>
                                <td>
                                    <div class="resource-action-strip">
                                        <button
                                            type="button"
                                            class="table-action-button"
                                            title="View resource details"
                                            aria-label="View resource details"
                                            on:click=move |event: MouseEvent| {
                                                event.stop_propagation();
                                                on_view_resource.call(view_resource.clone());
                                            }
                                        >
                                            <Icon icon=LuEye width="1em" height="1em" />
                                            <span class="sr-only">"View"</span>
                                        </button>
                                        {actions.into_iter().map(|action| {
                                            let action_resource = resource.clone();
                                            if !(action.visible)(&action_resource) {
                                                return view! { <></> }.into_view();
                                            }
                                            let disabled = (action.disabled)(&action_resource);
                                            let title = (action.title)(&action_resource);
                                            let class_name = row_action_class(action.tone);
                                            let icon = action.icon;
                                            let label = action.label.clone();
                                            view! {
                                                <button
                                                    type="button"
                                                    class=class_name
                                                    title=title.clone()
                                                    aria-label=title
                                                    disabled=disabled
                                                    on:click=move |event: MouseEvent| {
                                                        event.stop_propagation();
                                                        if !disabled {
                                                            action.on_click.call(action_resource.clone());
                                                        }
                                                    }
                                                >
                                                    <Icon icon=icon width="1em" height="1em" />
                                                    <span class="sr-only">{label}</span>
                                                </button>
                                            }
                                            .into_view()
                                        }).collect_view()}
                                    </div>
                                </td>
                            </tr>
                        }
                    }).collect_view()}
                </tbody>
            </table>
        </div>
    }
    .into_view()
}

fn attribute_summary_text(resource: &ResourceSummary) -> String {
    let mut parts = resource
        .attributes
        .iter()
        .take(2)
        .map(|(key, value)| format!("{key}: {value}"))
        .collect::<Vec<_>>();
    let remaining = resource.attributes.len().saturating_sub(2);

    if remaining > 0 {
        parts.push(format!("+{remaining}"));
    }

    if parts.is_empty() {
        "No attributes".to_owned()
    } else {
        parts.join(" · ")
    }
}

fn row_action_class(tone: ResourceRowActionTone) -> &'static str {
    match tone {
        ResourceRowActionTone::Neutral => "table-action-button",
        ResourceRowActionTone::Danger => "table-action-button danger",
        ResourceRowActionTone::Muted => "table-action-button muted",
    }
}

fn sort_header(
    label: &'static str,
    icon: IconData,
    column: ResourceSortColumn,
    sort_column: ReadSignal<ResourceSortColumn>,
    ascending: ReadSignal<bool>,
    change_sort: impl Fn(ResourceSortColumn) + Copy + 'static,
) -> View {
    view! {
        <button
            type="button"
            class="resource-sort-button"
            aria-sort=move || {
                if sort_column.get() != column {
                    "none"
                } else if ascending.get() {
                    "ascending"
                } else {
                    "descending"
                }
            }
            on:click=move |_| change_sort(column)
        >
            <Icon icon=icon width="1em" height="1em" />
            {label}
            <span aria-hidden="true">
                {move || {
                    let icon = if sort_column.get() != column {
                        LuArrowUpDown
                    } else if ascending.get() {
                        LuChevronUp
                    } else {
                        LuChevronDown
                    };
                    view! { <Icon icon=icon width="1em" height="1em" /> }
                }}
            </span>
        </button>
    }
    .into_view()
}

fn resource_kind_icon(kind: &str) -> IconData {
    match kind {
        "alarm" => LuAlertTriangle,
        "api" | "rest-api" => LuWifi,
        "application" | "project" => LuPackageSearch,
        "change-set" | "dead-letter-queue" | "image" | "repository" | "snapshot" => LuArchive,
        "database" | "db-instance" | "model" | "query-result" => LuDatabase,
        "dashboard" => LuLayoutDashboard,
        "file-system" => LuFolder,
        "health-check" => LuActivity,
        "integration" | "listener" | "listener-rule" | "method" | "route" => LuCable,
        "access-key" | "host-key" | "key" | "key-pair" | "key-policy" => LuKeyRound,
        "alias" | "app-client" | "connection" | "deployment-group" | "identity-provider"
        | "instance-profile" | "lifecycle-hook" | "scaling-policy" | "subscription" | "target" => {
            LuLink2
        }
        "account-setting" | "auto-scaling-group" | "broker" | "cache-cluster"
        | "container-instance" | "instance" | "load-balancer" | "node-group"
        | "replication-group" | "scaling-instance" | "server" | "stack" | "stack-event"
        | "stack-resource" | "state-machine" | "stream" | "target-group" => LuServer,
        "addon" | "bucket" | "function" | "group" | "managed-instance" | "message" | "queue"
        | "role" | "task" | "task-definition" | "user" | "user-pool" => LuPackage,
        "build" | "change-batch" | "deployment" | "invocation" | "replay" | "report" => LuRefreshCw,
        "authorizer" | "certificate" | "caller-identity" | "secret" | "secret-version" => {
            LuShieldCheck
        }
        "domain" | "event-bus" | "hosted-zone" | "rule" | "security-group" | "subnet" | "topic"
        | "vpc" => LuGlobe,
        "metric" | "send-statistic" | "table" | "tag" | "template" | "version"
        | "hosted-version" => LuTable2,
        "metric-filter" | "parameter" | "policy" | "queue-attribute" | "subscription-filter" => {
            LuTags
        }
        "record-set" => LuMapPin,
        "schedule" | "schedule-group" => LuCalendar,
        "stage" => LuCloudCog,
        "validation" => LuCheckCircle2,
        "volume" => LuHardDrive,
        _ => LuFileJson,
    }
}

fn resource_status_icon(status: &str) -> IconData {
    match status.to_ascii_lowercase().as_str() {
        "available" | "active" | "healthy" | "ok" | "running" => LuCheckCircle2,
        "blocked" | "deleting" | "error" | "failed" | "unavailable" => LuAlertTriangle,
        _ => LuCircleDot,
    }
}

fn resource_status_class(status: &str) -> &'static str {
    match status.to_ascii_lowercase().as_str() {
        "available" | "active" | "healthy" | "ok" | "running" => {
            "status-pill resource-status-pill status-up"
        }
        "blocked" | "deleting" | "error" | "failed" | "unavailable" => {
            "status-pill resource-status-pill status-down"
        }
        _ => "status-pill resource-status-pill status-neutral",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::service_management::models::ResourceSummary;

    use super::attribute_summary_text;

    fn resource_with_attributes(attributes: BTreeMap<String, String>) -> ResourceSummary {
        ResourceSummary {
            id: "bucket-1".to_owned(),
            name: "bucket-1".to_owned(),
            kind: "bucket".to_owned(),
            status: "available".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        }
    }

    #[test]
    fn summarizes_first_two_attributes_with_remaining_count() {
        let attributes = BTreeMap::from([
            ("objects".to_owned(), "18".to_owned()),
            ("region".to_owned(), "us-east-1".to_owned()),
            ("versioning".to_owned(), "disabled".to_owned()),
        ]);

        assert_eq!(
            attribute_summary_text(&resource_with_attributes(attributes)),
            "objects: 18 · region: us-east-1 · +1"
        );
    }

    #[test]
    fn summarizes_empty_attributes_without_layout_noise() {
        assert_eq!(
            attribute_summary_text(&resource_with_attributes(BTreeMap::new())),
            "No attributes"
        );
    }
}
