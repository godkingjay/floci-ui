#![allow(clippy::must_use_candidate, clippy::wildcard_imports)]

use icondata::{
    Icon as IconData, LuAlertTriangle, LuCheckCircle2, LuInfo, LuPackageOpen, LuShieldAlert, LuX,
};
use leptos::{ev::KeyboardEvent, ev::MouseEvent, *};
use leptos_icons::Icon;

use super::button::{Button, ButtonSize, ButtonVariant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NoticeTone {
    Info,
    Success,
    Warning,
    Danger,
}

impl Default for NoticeTone {
    fn default() -> Self {
        Self::Info
    }
}

impl NoticeTone {
    fn class_name(self) -> &'static str {
        match self {
            Self::Info => "notice-info",
            Self::Success => "notice-success",
            Self::Warning => "notice-warning",
            Self::Danger => "notice-danger",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Toast {
    pub id: String,
    pub message: String,
    pub tone: NoticeTone,
}

#[component]
pub fn EmptyState(
    #[prop(into)] title: String,
    #[prop(into)] message: String,
    #[prop(optional)] action: Option<View>,
    #[prop(optional)] icon: Option<IconData>,
) -> impl IntoView {
    let icon = icon.unwrap_or(LuPackageOpen);

    view! {
        <div class="empty-state">
            <span class="empty-state-icon" aria-hidden="true">
                <Icon icon=icon width="1em" height="1em" />
            </span>
            <h3>{title}</h3>
            <p>{message}</p>
            {action}
        </div>
    }
}

#[component]
pub fn ErrorState(
    #[prop(into)] title: String,
    #[prop(into)] message: String,
    #[prop(optional)] action: Option<View>,
    #[prop(optional)] icon: Option<IconData>,
) -> impl IntoView {
    let icon = icon.unwrap_or(LuAlertTriangle);

    view! {
        <div class="error-state" role="alert">
            <span class="error-state-icon" aria-hidden="true">
                <Icon icon=icon width="1em" height="1em" />
            </span>
            <h3>{title}</h3>
            <p>{message}</p>
            {action}
        </div>
    }
}

#[component]
pub fn InlineNotice(
    children: Children,
    #[prop(optional)] tone: NoticeTone,
    #[prop(optional, into)] title: Option<String>,
) -> impl IntoView {
    let class_name = format!("inline-notice {}", tone.class_name());
    let icon = notice_icon(tone);

    view! {
        <div class=class_name>
            {title.map(|title| view! {
                <div class="inline-notice-title">
                    <Icon icon=icon width="1em" height="1em" />
                    <strong>{title}</strong>
                </div>
            })}
            <div>{children()}</div>
        </div>
    }
}

#[component]
pub fn ToastRegion(toasts: Vec<Toast>) -> impl IntoView {
    view! {
        <div class="toast-region" role="status" aria-live="polite">
            {toasts.into_iter().map(|toast| {
                let class_name = format!("toast {}", toast.tone.class_name());
                view! {
                    <div id=toast.id class=class_name>{toast.message}</div>
                }
            }).collect_view()}
        </div>
    }
}

#[component]
pub fn Dialog(
    children: Children,
    #[prop(into)] title: String,
    #[prop(optional, into)] eyebrow: String,
    #[prop(optional, into)] description: String,
    #[prop(optional)] footer: Option<View>,
    #[prop(optional)] open: bool,
    #[prop(optional, into)] loading: MaybeSignal<bool>,
    #[prop(default = Callback::new(|_: ()| {}))] on_close: Callback<()>,
) -> impl IntoView {
    #[cfg(target_arch = "wasm32")]
    let restore_focus_target = active_dialog_trigger();
    #[cfg(target_arch = "wasm32")]
    let backdrop_focus_target = restore_focus_target.clone();
    #[cfg(target_arch = "wasm32")]
    let keyboard_focus_target = restore_focus_target.clone();
    #[cfg(target_arch = "wasm32")]
    let close_button_focus_target = restore_focus_target.clone();
    #[cfg(target_arch = "wasm32")]
    let window_focus_target = restore_focus_target.clone();

    let close_from_backdrop = move |event: MouseEvent| {
        event.stop_propagation();
        if !loading.get_untracked() {
            #[cfg(target_arch = "wasm32")]
            restore_dialog_focus(&backdrop_focus_target);
            on_close.call(());
        }
    };
    let close_from_keyboard = move |event: KeyboardEvent| {
        if event.key() == "Escape" && !loading.get_untracked() {
            #[cfg(target_arch = "wasm32")]
            restore_dialog_focus(&keyboard_focus_target);
            on_close.call(());
        }
    };
    let body = children();

    #[cfg(target_arch = "wasm32")]
    window_event_listener(ev::keydown, move |event: KeyboardEvent| {
        if open && event.key() == "Escape" && !loading.get_untracked() {
            restore_dialog_focus(&window_focus_target);
            on_close.call(());
        }
    });

    view! {
        <div
            class="dialog-backdrop"
            hidden=!open
            tabindex="-1"
            on:click=close_from_backdrop
            on:keydown=close_from_keyboard
        >
            <section
                class="dialog-panel"
                role="dialog"
                aria-modal="true"
                aria-labelledby="dialog-title"
                aria-busy=move || loading.get().to_string()
                on:click=|event: MouseEvent| event.stop_propagation()
            >
                <header class="dialog-header">
                    <div class="dialog-title-group">
                        {(!eyebrow.is_empty()).then(|| view! { <p class="eyebrow">{eyebrow}</p> })}
                        <h2 id="dialog-title">{title}</h2>
                        {(!description.is_empty()).then(|| view! { <p>{description}</p> })}
                    </div>
                    <button
                        type="button"
                        class="dialog-close"
                        aria-label="Close dialog"
                        disabled=move || loading.get()
                        on:click=move |_| {
                            if !loading.get_untracked() {
                                #[cfg(target_arch = "wasm32")]
                                restore_dialog_focus(&close_button_focus_target);
                                on_close.call(());
                            }
                        }
                    >
                        <Icon icon=LuX width="1em" height="1em" />
                    </button>
                </header>
                <div class="dialog-body">{body}</div>
                {footer.map(|footer| view! { <footer class="dialog-footer">{footer}</footer> })}
            </section>
        </div>
    }
}

fn notice_icon(tone: NoticeTone) -> IconData {
    match tone {
        NoticeTone::Info => LuInfo,
        NoticeTone::Success => LuCheckCircle2,
        NoticeTone::Warning => LuAlertTriangle,
        NoticeTone::Danger => LuShieldAlert,
    }
}

#[cfg(target_arch = "wasm32")]
fn active_dialog_trigger() -> Option<web_sys::Element> {
    web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.active_element())
}

#[cfg(target_arch = "wasm32")]
fn restore_dialog_focus(target: &Option<web_sys::Element>) {
    use wasm_bindgen::JsCast;

    if let Some(element) = target {
        if let Some(html_element) = element.dyn_ref::<web_sys::HtmlElement>() {
            let _ = html_element.focus();
        }
    }
}

#[component]
pub fn ConfirmDialog(
    #[prop(into)] title: String,
    #[prop(into)] message: String,
    #[prop(optional, into)] confirm_label: String,
    #[prop(optional)] open: bool,
    #[prop(optional)] tone: NoticeTone,
    #[prop(optional, into)] loading: MaybeSignal<bool>,
    #[prop(optional, into)] required_confirmation: String,
    #[prop(optional, into)] error: String,
    #[prop(default = Callback::new(|_: ()| {}))] on_cancel: Callback<()>,
    #[prop(default = Callback::new(|_: String| {}))] on_confirm: Callback<String>,
) -> impl IntoView {
    let confirm_text = if confirm_label.is_empty() {
        "Confirm".to_owned()
    } else {
        confirm_label
    };
    let (confirmation, set_confirmation) = create_signal(String::new());
    let confirmation_requirement = required_confirmation.clone();
    let can_confirm = move || {
        confirmation_requirement.is_empty() || confirmation.get() == confirmation_requirement
    };
    let confirm_variant = if matches!(tone, NoticeTone::Danger) {
        ButtonVariant::Danger
    } else {
        ButtonVariant::Primary
    };
    let footer = view! {
        <div class="dialog-actions">
            <Button
                variant=ButtonVariant::Secondary
                size=ButtonSize::Medium
                disabled=loading
                on_click=Callback::new(move |_| on_cancel.call(()))
            >
                "Cancel"
            </Button>
            <Button
                variant=confirm_variant
                size=ButtonSize::Medium
                loading=loading
                disabled=Signal::derive(move || !can_confirm())
                on_click=Callback::new(move |_| on_confirm.call(confirmation.get_untracked()))
            >
                {confirm_text}
            </Button>
        </div>
    }
    .into_view();

    view! {
        <Dialog
            title=title
            open=open
            loading=loading
            footer=footer
            on_close=on_cancel
        >
            <div class="confirm-dialog-body">
                <p>{message}</p>
                {(!required_confirmation.is_empty()).then(|| view! {
                    <label class="confirmation-field">
                        <span>{format!("Type `{required_confirmation}` to confirm")}</span>
                        <input
                            type="text"
                            autocomplete="off"
                            value=confirmation
                            on:input=move |event| set_confirmation.set(event_target_value(&event))
                        />
                    </label>
                })}
                {(!error.is_empty()).then(|| view! {
                    <div class="dialog-error" role="alert">{error}</div>
                })}
            </div>
        </Dialog>
    }
}
