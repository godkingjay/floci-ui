#![allow(clippy::must_use_candidate, clippy::wildcard_imports)]

use icondata::{Icon as IconData, LuAlertTriangle, LuCheckCircle2, LuCircleDot, LuCircleSlash};
use leptos::*;
use leptos_icons::Icon;

use crate::models::ServiceCategory;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BadgeTone {
    Neutral,
    Success,
    Warning,
    Danger,
    Info,
}

impl Default for BadgeTone {
    fn default() -> Self {
        Self::Neutral
    }
}

impl BadgeTone {
    fn class_name(self) -> &'static str {
        match self {
            Self::Neutral => "ui-badge-neutral",
            Self::Success => "ui-badge-success",
            Self::Warning => "ui-badge-warning",
            Self::Danger => "ui-badge-danger",
            Self::Info => "ui-badge-info",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatusTone {
    Up,
    Down,
    Neutral,
    Muted,
}

impl Default for StatusTone {
    fn default() -> Self {
        Self::Neutral
    }
}

impl StatusTone {
    fn class_name(self) -> &'static str {
        match self {
            Self::Up => "status-up",
            Self::Down => "status-down",
            Self::Neutral => "status-neutral",
            Self::Muted => "status-muted",
        }
    }
}

#[component]
pub fn Badge(children: Children, #[prop(optional)] tone: BadgeTone) -> impl IntoView {
    let class_name = format!("ui-badge {}", tone.class_name());

    view! {
        <span class=class_name>{children()}</span>
    }
}

#[component]
pub fn StatusBadge(
    #[prop(into)] label: String,
    #[prop(optional)] tone: StatusTone,
    #[prop(optional)] icon: Option<IconData>,
) -> impl IntoView {
    let class_name = format!("status-pill {}", tone.class_name());
    let icon = icon.unwrap_or_else(|| status_icon(tone));

    view! {
        <span class=class_name>
            <Icon icon=icon width="0.85em" height="0.85em" />
            {label}
        </span>
    }
}

#[component]
pub fn ServiceCategoryBadge(category: ServiceCategory) -> impl IntoView {
    let class_name = format!("category {}", category_class(category));

    view! {
        <span class=class_name>{category_label(category)}</span>
    }
}

fn category_class(category: ServiceCategory) -> &'static str {
    match category {
        ServiceCategory::Core => "core",
        ServiceCategory::Compute => "compute",
        ServiceCategory::Data => "data",
        ServiceCategory::Messaging => "messaging",
        ServiceCategory::Networking => "networking",
        ServiceCategory::Observability => "observability",
        ServiceCategory::Security => "security",
    }
}

fn category_label(category: ServiceCategory) -> &'static str {
    match category {
        ServiceCategory::Core => "Core",
        ServiceCategory::Compute => "Compute",
        ServiceCategory::Data => "Data",
        ServiceCategory::Messaging => "Messaging",
        ServiceCategory::Networking => "Networking",
        ServiceCategory::Observability => "Observability",
        ServiceCategory::Security => "Security",
    }
}

fn status_icon(tone: StatusTone) -> IconData {
    match tone {
        StatusTone::Up => LuCheckCircle2,
        StatusTone::Down => LuAlertTriangle,
        StatusTone::Neutral => LuCircleDot,
        StatusTone::Muted => LuCircleSlash,
    }
}
