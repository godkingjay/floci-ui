#![allow(
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::wildcard_imports
)]

use icondata::{LuClock, LuGlobe, LuKeyRound, LuMapPin, LuServerCog, LuShieldCheck};
use leptos::*;
use leptos_icons::Icon;

use crate::{
    components::{
        InlineNotice, MetricTile, NoticeTone, Panel, PanelHeader, StatusBadge, StatusTone,
    },
    models::DashboardSnapshot,
    state::{AppStore, RemoteData},
};

#[component]
pub fn SettingsView(store: AppStore) -> impl IntoView {
    view! {
        <div class="view-stack">
            <section class="section-heading">
                <p class="eyebrow">"Settings"</p>
                <h2>"Runtime connection"</h2>
            </section>
            {move || render_settings(store.catalog.get())}
        </div>
    }
}

fn render_settings(state: RemoteData<DashboardSnapshot>) -> View {
    match state {
        RemoteData::Loading => view! {
            <Panel class="status-panel">
                <PanelHeader title="Loading runtime settings" />
                <p class="muted-text">"Reading local emulator configuration."</p>
            </Panel>
        }
        .into_view(),
        RemoteData::Failed(message) => view! {
            <Panel class="status-panel">
                <PanelHeader title="Runtime settings unavailable" />
                <p class="error-text">{message}</p>
            </Panel>
        }
        .into_view(),
        RemoteData::Ready(snapshot) => settings_panel(snapshot),
    }
}

fn settings_panel(snapshot: DashboardSnapshot) -> View {
    let credential_tone = if snapshot
        .credentials_status
        .eq_ignore_ascii_case("configured")
    {
        StatusTone::Up
    } else {
        StatusTone::Down
    };
    let endpoint_url = snapshot.endpoint_url.clone();
    let endpoint_host = snapshot.endpoint_host();
    let region = snapshot.region.clone();
    let access_key_id = snapshot.access_key_id.clone();
    let credentials_status = snapshot.credentials_status.clone();
    let last_refreshed_at = snapshot.last_refreshed_at.clone();
    let access_key_summary = access_key_id.clone();
    let access_key_metric = access_key_id;
    let credentials_status_badge = credentials_status.clone();
    let credentials_status_metric = credentials_status;

    view! {
        <section class="settings-grid">
            <Panel class="status-panel">
                <div class="panel-status-row">
                    <StatusBadge tone=StatusTone::Neutral label="Local only" />
                    <StatusBadge tone=credential_tone label=credentials_status_badge />
                </div>
                <PanelHeader
                    title="Connection"
                    icon=LuServerCog
                    description="These values are read from the local Tauri runtime and environment."
                />
                <dl class="summary-list">
                    <div>
                        <dt><span class="definition-label"><Icon icon=LuGlobe width="1em" height="1em" />"Endpoint URL"</span></dt>
                        <dd>{endpoint_url}</dd>
                    </div>
                    <div>
                        <dt><span class="definition-label"><Icon icon=LuServerCog width="1em" height="1em" />"Endpoint host"</span></dt>
                        <dd>{endpoint_host}</dd>
                    </div>
                    <div>
                        <dt><span class="definition-label"><Icon icon=LuMapPin width="1em" height="1em" />"Region"</span></dt>
                        <dd>{region}</dd>
                    </div>
                    <div>
                        <dt><span class="definition-label"><Icon icon=LuKeyRound width="1em" height="1em" />"Access key"</span></dt>
                        <dd>{access_key_summary}</dd>
                    </div>
                    <div>
                        <dt><span class="definition-label"><Icon icon=LuClock width="1em" height="1em" />"Last refreshed"</span></dt>
                        <dd>{last_refreshed_at}</dd>
                    </div>
                </dl>
            </Panel>

            <Panel class="status-panel compact">
                <PanelHeader title="Credential status" icon=LuShieldCheck />
                <div class="data-grid runtime-data-grid">
                    <MetricTile label="Status" value=credentials_status_metric icon=LuShieldCheck />
                    <MetricTile label="Access key" value=access_key_metric icon=LuKeyRound />
                </div>
                <InlineNotice tone=NoticeTone::Info title="Secret key is intentionally hidden">
                    <p>
                        "The UI only receives the access key label and credential status. Secret access keys stay inside the local runtime process and are not serialized into dashboard snapshots."
                    </p>
                </InlineNotice>
            </Panel>
        </section>
    }
    .into_view()
}
