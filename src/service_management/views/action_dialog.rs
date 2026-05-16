#![allow(clippy::must_use_candidate, clippy::wildcard_imports)]

use icondata::{LuCircleSlash, LuShieldCheck};
use leptos::*;
use leptos_icons::Icon;

use crate::{
    components::{InlineNotice, NoticeTone, Panel, PanelHeader},
    service_management::models::ServiceSupportLevel,
};

#[component]
pub fn ActionDialog(
    #[prop(into)] service_label: String,
    support_level: ServiceSupportLevel,
    safe_operations: Vec<String>,
    unsupported_operations: Vec<String>,
) -> impl IntoView {
    let operations = if safe_operations.is_empty() {
        vec!["adapter pending".to_owned()]
    } else {
        safe_operations
    };
    let unsupported_text = if unsupported_operations.is_empty() {
        "No blocked operations were reported.".to_owned()
    } else {
        unsupported_operations.join(", ")
    };

    view! {
        <Panel class="status-panel compact">
            <PanelHeader
                title="Safe actions"
                icon=LuShieldCheck
                description=format!("{service_label} action contract")
            />
            {matches!(support_level, ServiceSupportLevel::Unsupported).then(|| view! {
                <InlineNotice tone=NoticeTone::Warning title="Adapter pending">
                    "This descriptor is registered, but inventory and actions are disabled until a service adapter is added."
                </InlineNotice>
            })}
            <div class="safe-action-toolbar">
                {operations.into_iter().map(|operation| view! {
                    <button type="button" class="safe-action-button disabled" disabled=true>
                        <Icon icon=LuCircleSlash width="1em" height="1em" />
                        {operation_label(&operation)}
                    </button>
                }).collect_view()}
            </div>
            <dl class="summary-list">
                <div>
                    <dt>"Blocked"</dt>
                    <dd>{unsupported_text}</dd>
                </div>
            </dl>
        </Panel>
    }
}

fn operation_label(operation: &str) -> String {
    operation
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}
