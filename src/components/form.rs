#![allow(clippy::must_use_candidate, clippy::wildcard_imports)]

use leptos::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectOption {
    pub label: String,
    pub value: String,
}

#[component]
pub fn FieldError(#[prop(into)] message: String) -> impl IntoView {
    view! {
        <p class="field-error" role="alert">{message}</p>
    }
}

#[component]
pub fn TextInput(
    #[prop(into)] id: String,
    #[prop(into)] label: String,
    #[prop(optional, into)] value: String,
    #[prop(optional, into)] placeholder: String,
    #[prop(optional)] disabled: bool,
    #[prop(default = Callback::new(|_: String| {}))] on_input: Callback<String>,
) -> impl IntoView {
    view! {
        <label class="form-field" for=id.clone()>
            <span class="field-label">{label}</span>
            <input
                id=id
                class="field-control"
                type="text"
                prop:value=value
                placeholder=placeholder
                disabled=disabled
                on:input=move |event| on_input.call(event_target_value(&event))
            />
        </label>
    }
}

#[component]
pub fn SearchInput(
    #[prop(into)] id: String,
    #[prop(optional, into)] value: String,
    #[prop(optional, into)] placeholder: String,
    #[prop(default = Callback::new(|_: String| {}))] on_input: Callback<String>,
) -> impl IntoView {
    view! {
        <label class="form-field search-field" for=id.clone()>
            <span class="sr-only">"Search"</span>
            <input
                id=id
                class="field-control"
                type="search"
                prop:value=value
                placeholder=placeholder
                on:input=move |event| on_input.call(event_target_value(&event))
            />
        </label>
    }
}

#[component]
pub fn SelectInput(
    #[prop(into)] id: String,
    #[prop(into)] label: String,
    options: Vec<SelectOption>,
    #[prop(optional, into)] value: String,
    #[prop(default = Callback::new(|_: String| {}))] on_change: Callback<String>,
) -> impl IntoView {
    view! {
        <label class="form-field" for=id.clone()>
            <span class="field-label">{label}</span>
            <select
                id=id
                class="field-control"
                prop:value=value
                on:change=move |event| on_change.call(event_target_value(&event))
            >
                {options.into_iter().map(|option| {
                    view! {
                        <option value=option.value>{option.label}</option>
                    }
                }).collect_view()}
            </select>
        </label>
    }
}

#[component]
pub fn ToggleSwitch(
    #[prop(into)] id: String,
    #[prop(into)] label: String,
    checked: bool,
    #[prop(default = Callback::new(|_: bool| {}))] on_change: Callback<bool>,
) -> impl IntoView {
    view! {
        <label class="toggle-switch" for=id.clone()>
            <input
                id=id
                type="checkbox"
                checked=checked
                on:change=move |event| on_change.call(event_target_checked(&event))
            />
            <span class="toggle-track" aria-hidden="true">
                <span class="toggle-thumb"></span>
            </span>
            <span class="field-label">{label}</span>
        </label>
    }
}
