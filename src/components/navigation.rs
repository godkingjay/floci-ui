#![allow(
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::wildcard_imports
)]

use leptos::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavItem {
    pub label: String,
    pub href: String,
    pub active: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BreadcrumbItem {
    pub label: String,
    pub href: Option<String>,
}

#[component]
pub fn SidebarNav(items: Vec<NavItem>, #[prop(optional, into)] label: String) -> impl IntoView {
    let nav_label = if label.is_empty() {
        "Primary navigation".to_owned()
    } else {
        label
    };

    view! {
        <nav class="app-sidebar" aria-label=nav_label>
            {items.into_iter().map(|item| {
                let class_name = if item.active { "nav-link active" } else { "nav-link" };
                view! {
                    <a class=class_name href=item.href aria-current=item.active.then_some("page")>
                        {item.label}
                    </a>
                }
            }).collect_view()}
        </nav>
    }
}

#[component]
pub fn Breadcrumbs(items: Vec<BreadcrumbItem>) -> impl IntoView {
    view! {
        <nav class="breadcrumbs" aria-label="Breadcrumb">
            <ol>
                {items.into_iter().map(|item| view! {
                    <li>
                        {match item.href {
                            Some(href) => view! { <a href=href>{item.label}</a> }.into_view(),
                            None => view! { <span aria-current="page">{item.label}</span> }.into_view(),
                        }}
                    </li>
                }).collect_view()}
            </ol>
        </nav>
    }
}

#[component]
pub fn TopBar(children: Children, #[prop(optional, into)] class: String) -> impl IntoView {
    let class_name = format!("top-bar {class}");

    view! {
        <header class=class_name>{children()}</header>
    }
}

#[component]
pub fn CommandBar(children: Children, #[prop(optional, into)] class: String) -> impl IntoView {
    let class_name = format!("command-bar toolbar-row {class}");

    view! {
        <div class=class_name role="toolbar">{children()}</div>
    }
}
