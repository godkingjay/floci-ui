#![allow(
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::wildcard_imports
)]

use leptos::{ev::MouseEvent, *};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

impl Default for ButtonVariant {
    fn default() -> Self {
        Self::Secondary
    }
}

impl ButtonVariant {
    fn class_name(self) -> &'static str {
        match self {
            Self::Primary => "ui-button-primary",
            Self::Secondary => "ui-button-secondary",
            Self::Ghost => "ui-button-ghost",
            Self::Danger => "ui-button-danger",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ButtonSize {
    Small,
    Medium,
    Large,
}

impl Default for ButtonSize {
    fn default() -> Self {
        Self::Medium
    }
}

impl ButtonSize {
    fn class_name(self) -> &'static str {
        match self {
            Self::Small => "ui-button-small",
            Self::Medium => "ui-button-medium",
            Self::Large => "ui-button-large",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ButtonType {
    Button,
    Submit,
    Reset,
}

impl Default for ButtonType {
    fn default() -> Self {
        Self::Button
    }
}

impl ButtonType {
    fn attribute_value(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Submit => "submit",
            Self::Reset => "reset",
        }
    }
}

#[component]
pub fn Button(
    children: Children,
    #[prop(optional)] variant: ButtonVariant,
    #[prop(optional)] size: ButtonSize,
    #[prop(optional)] button_type: ButtonType,
    #[prop(optional, into)] loading: MaybeSignal<bool>,
    #[prop(optional, into)] disabled: MaybeSignal<bool>,
    #[prop(optional, into)] class: String,
    #[prop(optional)] icon: Option<View>,
    #[prop(default = Callback::new(|_: MouseEvent| {}))] on_click: Callback<MouseEvent>,
) -> impl IntoView {
    let class_name = format!(
        "ui-button {} {} {}",
        variant.class_name(),
        size.class_name(),
        class
    );
    let content = children();

    view! {
        <button
            type=button_type.attribute_value()
            class=class_name
            disabled=move || disabled.get() || loading.get()
            aria-busy=move || loading.get().to_string()
            on:click=move |event| {
                if !disabled.get_untracked() && !loading.get_untracked() {
                    on_click.call(event);
                }
            }
        >
            {icon.map(|icon| view! { <span class="ui-button-icon">{icon}</span> })}
            <span class="ui-button-content">{content}</span>
            {move || loading.get().then(|| view! {
                <span class="ui-button-spinner" aria-hidden="true"></span>
            })}
        </button>
    }
}
