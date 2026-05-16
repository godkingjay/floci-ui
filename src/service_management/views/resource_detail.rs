#![allow(clippy::must_use_candidate, clippy::wildcard_imports)]

use icondata::{
    Icon as IconData, LuCalendar, LuChevronDown, LuChevronUp, LuClock, LuCopy, LuDatabase,
    LuFileJson, LuFingerprint, LuLink2, LuTags,
};
use leptos::*;
use leptos_icons::Icon;
use wasm_bindgen_futures::spawn_local;

use crate::{
    components::{Dialog, InlineNotice, NoticeTone, Panel, PanelHeader},
    service_management::models::{ResourceDetail, ResourceRelationship, ResourceSummary},
    state::RemoteData,
};

#[component]
pub fn ResourceDetailPanel(selected: Option<ResourceSummary>) -> impl IntoView {
    match selected {
        Some(resource) => {
            let attributes = resource.attributes.clone();
            view! {
                <Panel class="status-panel compact">
                    <PanelHeader
                        title=resource.name.clone()
                        icon=LuDatabase
                        eyebrow="Resource detail"
                        description=format!("{} · {}", resource.kind, resource.status)
                    />
                    <dl class="summary-list">
                        <div>
                            <dt>"Resource ID"</dt>
                            <dd><code>{resource.id}</code></dd>
                        </div>
                        <div>
                            <dt>"Created"</dt>
                            <dd>{resource.created_at.unwrap_or_else(|| "n/a".to_owned())}</dd>
                        </div>
                        <div>
                            <dt>"Updated"</dt>
                            <dd>{resource.updated_at.unwrap_or_else(|| "n/a".to_owned())}</dd>
                        </div>
                    </dl>
                    <div class="attribute-grid">
                        {attributes.into_iter().map(|(key, value)| view! {
                            <div>
                                <span>{key}</span>
                                <strong>{value}</strong>
                            </div>
                        }).collect_view()}
                    </div>
                </Panel>
            }
            .into_view()
        }
        None => view! {
            <Panel class="status-panel compact">
                <PanelHeader title="Resource detail" icon=LuDatabase />
                <p class="muted-text">"Select a resource to inspect metadata and safe actions."</p>
            </Panel>
        }
        .into_view(),
    }
}

#[component]
pub fn JsonInspectorPanel(
    #[prop(into)] title: String,
    #[prop(into)] json: String,
    #[prop(optional)] icon: Option<IconData>,
    #[prop(optional, into)] description: String,
) -> impl IntoView {
    let (expanded, set_expanded) = create_signal(true);
    let (copy_status, set_copy_status) = create_signal(None::<String>);
    let panel_id = format!("json-inspector-{}-{}", stable_id(&title), json.len());
    let description = if description.is_empty() {
        "Raw inventory metadata excludes object bodies and secret values.".to_owned()
    } else {
        description
    };

    view! {
        <Panel class="status-panel compact">
            <PanelHeader
                title=title.clone()
                icon=icon.unwrap_or(LuFileJson)
                description=description
            />
            <div class="json-inspector-toolbar">
                <button
                    type="button"
                    class="json-inspector-button"
                    aria-label=format!("Copy {title} JSON")
                    on:click={
                        let json = json.clone();
                        move |_| {
                            let json = json.clone();
                            spawn_local(async move {
                                let message = match copy_text_to_clipboard(json).await {
                                    Ok(()) => "Copied JSON".to_owned(),
                                    Err(message) => message,
                                };
                                set_copy_status.set(Some(message));
                            });
                        }
                    }
                >
                    <Icon icon=LuCopy width="1em" height="1em" />
                    "Copy"
                </button>
                <button
                    type="button"
                    class="json-inspector-button"
                    aria-controls=panel_id.clone()
                    aria-expanded=move || expanded.get().to_string()
                    on:click=move |_| set_expanded.update(|value| *value = !*value)
                >
                    {move || if expanded.get() {
                        view! { <Icon icon=LuChevronUp width="1em" height="1em" /> }.into_view()
                    } else {
                        view! { <Icon icon=LuChevronDown width="1em" height="1em" /> }.into_view()
                    }}
                    {move || if expanded.get() { "Collapse" } else { "Expand" }}
                </button>
            </div>
            {move || copy_status.get().map(|message| view! {
                <p class="json-inspector-status" role="status">{message}</p>
            })}
            {move || expanded.get().then(|| view! {
                <pre id=panel_id.clone() class="json-inspector">{json.clone()}</pre>
            })}
        </Panel>
    }
}

fn stable_id(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(target_arch = "wasm32")]
async fn copy_text_to_clipboard(value: String) -> Result<(), String> {
    let clipboard = web_sys::window()
        .ok_or_else(|| "No browser window is available.".to_owned())?
        .navigator()
        .clipboard();
    wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&value))
        .await
        .map(|_| ())
        .map_err(|_| "Clipboard copy was blocked by the browser.".to_owned())
}

#[cfg(not(target_arch = "wasm32"))]
async fn copy_text_to_clipboard(_value: String) -> Result<(), String> {
    Err("Clipboard copy is only available in the browser.".to_owned())
}

#[component]
pub fn ResourceDetailDialog(
    #[prop(optional)] open: bool,
    selected: Option<ResourceSummary>,
    detail: RemoteData<ResourceDetail>,
    #[prop(default = Callback::new(|_: ()| {}))] on_close: Callback<()>,
) -> impl IntoView {
    let title = selected.as_ref().map_or_else(
        || "Resource detail".to_owned(),
        |resource| resource.name.clone(),
    );
    let description = selected.as_ref().map_or_else(String::new, |resource| {
        format!("{} · {}", resource.kind, resource.status)
    });

    view! {
        <Dialog
            title=title
            eyebrow="Resource detail"
            description=description
            open=open
            on_close=on_close
        >
            {match selected {
                Some(resource) => render_dialog_body(resource, detail),
                None => view! {
                    <p class="muted-text">"Choose a resource to inspect metadata."</p>
                }.into_view(),
            }}
        </Dialog>
    }
}

fn render_dialog_body(resource: ResourceSummary, detail: RemoteData<ResourceDetail>) -> View {
    match detail {
        RemoteData::Loading => view! {
            <div class="resource-dialog-content">
                {render_summary_fields(resource)}
                <InlineNotice tone=NoticeTone::Info title="Loading metadata">
                    "Fetching resource detail from the Tauri backend."
                </InlineNotice>
            </div>
        }
        .into_view(),
        RemoteData::Failed(message) => view! {
            <div class="resource-dialog-content">
                {render_summary_fields(resource)}
                <InlineNotice tone=NoticeTone::Danger title="Detail unavailable">
                    {message}
                </InlineNotice>
            </div>
        }
        .into_view(),
        RemoteData::Ready(detail) => {
            let metadata =
                serde_json::to_string_pretty(&detail.metadata).unwrap_or_else(|_| "{}".to_owned());
            view! {
                <div class="resource-dialog-content">
                    {render_summary_fields(detail.summary.clone())}
                    {render_key_value_section("Attributes", detail.summary.attributes.clone(), "No attributes")}
                    {render_key_value_section("Tags", detail.summary.tags.clone(), "No tags")}
                    {render_relationships(detail.relationships)}
                    <section class="resource-modal-section">
                        <h3 class="section-title-with-icon">
                            <Icon icon=LuFileJson width="1em" height="1em" />
                            "JSON metadata"
                        </h3>
                        <pre class="json-inspector modal">{metadata}</pre>
                    </section>
                </div>
            }
            .into_view()
        }
    }
}

fn render_summary_fields(resource: ResourceSummary) -> View {
    view! {
        <section class="resource-modal-section">
            <h3 class="section-title-with-icon">
                <Icon icon=LuDatabase width="1em" height="1em" />
                "Summary"
            </h3>
            <dl class="summary-list resource-summary-grid">
                <div>
                    <dt><span class="definition-label"><Icon icon=LuFingerprint width="1em" height="1em" />"Resource ID"</span></dt>
                    <dd><code>{resource.id}</code></dd>
                </div>
                <div>
                    <dt><span class="definition-label"><Icon icon=LuDatabase width="1em" height="1em" />"Kind"</span></dt>
                    <dd>{resource.kind}</dd>
                </div>
                <div>
                    <dt><span class="definition-label"><Icon icon=LuTags width="1em" height="1em" />"Status"</span></dt>
                    <dd><span class="status-pill">{resource.status}</span></dd>
                </div>
                <div>
                    <dt><span class="definition-label"><Icon icon=LuCalendar width="1em" height="1em" />"Created"</span></dt>
                    <dd>{resource.created_at.unwrap_or_else(|| "n/a".to_owned())}</dd>
                </div>
                <div>
                    <dt><span class="definition-label"><Icon icon=LuClock width="1em" height="1em" />"Updated"</span></dt>
                    <dd>{resource.updated_at.unwrap_or_else(|| "n/a".to_owned())}</dd>
                </div>
            </dl>
        </section>
    }
    .into_view()
}

fn render_key_value_section(
    title: &'static str,
    values: std::collections::BTreeMap<String, String>,
    empty_message: &'static str,
) -> View {
    view! {
        <section class="resource-modal-section">
            <h3 class="section-title-with-icon">
                <Icon icon=LuTags width="1em" height="1em" />
                {title}
            </h3>
            {if values.is_empty() {
                view! { <p class="muted-text">{empty_message}</p> }.into_view()
            } else {
                view! {
                    <div class="attribute-grid modal">
                        {values.into_iter().map(|(key, value)| view! {
                            <div>
                                <span>{key}</span>
                                <strong>{value}</strong>
                            </div>
                        }).collect_view()}
                    </div>
                }.into_view()
            }}
        </section>
    }
    .into_view()
}

fn render_relationships(relationships: Vec<ResourceRelationship>) -> View {
    view! {
        <section class="resource-modal-section">
            <h3 class="section-title-with-icon">
                <Icon icon=LuLink2 width="1em" height="1em" />
                "Relationships"
            </h3>
            {if relationships.is_empty() {
                view! { <p class="muted-text">"No relationships"</p> }.into_view()
            } else {
                view! {
                    <ul class="resource-relationship-list">
                        {relationships.into_iter().map(|relationship| view! {
                            <li>
                                <span>{relationship.label}</span>
                                <strong>{relationship.target}</strong>
                                <code>{relationship.kind}</code>
                            </li>
                        }).collect_view()}
                    </ul>
                }.into_view()
            }}
        </section>
    }
    .into_view()
}
