#![allow(
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::wildcard_imports
)]

use icondata::Icon as IconData;
use leptos::*;
use leptos_icons::Icon;

#[component]
pub fn Panel(children: Children, #[prop(optional, into)] class: String) -> impl IntoView {
    let class_name = format!("panel {class}");

    view! {
        <article class=class_name>{children()}</article>
    }
}

#[component]
pub fn PanelHeader(
    #[prop(into)] title: String,
    #[prop(optional)] icon: Option<IconData>,
    #[prop(optional, into)] eyebrow: Option<String>,
    #[prop(optional, into)] description: Option<String>,
) -> impl IntoView {
    view! {
        <header class="panel-header">
            <div class="panel-header-main">
                {icon.map(|icon| view! {
                    <span class="panel-header-icon" aria-hidden="true">
                        <Icon icon=icon width="1em" height="1em" />
                    </span>
                })}
                <div class="panel-header-copy">
                    {eyebrow.map(|eyebrow| view! { <p class="eyebrow">{eyebrow}</p> })}
                    <h2 class="panel-title">{title}</h2>
                    {description.map(|description| view! { <p class="panel-description">{description}</p> })}
                </div>
            </div>
        </header>
    }
}

#[component]
pub fn MetricTile(
    #[prop(into)] label: String,
    #[prop(into)] value: String,
    #[prop(optional, into)] detail: Option<String>,
    #[prop(optional)] icon: Option<IconData>,
) -> impl IntoView {
    view! {
        <div class="metric-tile">
            <div class="metric-tile-header">
                <span class="metric-label">{label}</span>
                {icon.map(|icon| view! {
                    <span class="metric-icon" aria-hidden="true">
                        <Icon icon=icon width="1em" height="1em" />
                    </span>
                })}
            </div>
            <strong class="metric-value">{value}</strong>
            {detail.map(|detail| view! { <span class="metric-detail">{detail}</span> })}
        </div>
    }
}

#[component]
pub fn SectionBand(
    children: Children,
    #[prop(optional, into)] class: String,
    #[prop(optional, into)] aria_label: Option<String>,
) -> impl IntoView {
    let class_name = format!("content-band {class}");

    view! {
        <section class=class_name aria-label=aria_label>
            {children()}
        </section>
    }
}
