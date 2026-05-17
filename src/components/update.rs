#![allow(clippy::must_use_candidate, clippy::wildcard_imports)]

use icondata::{LuDownload, LuPackageCheck, LuRefreshCw};
use leptos::*;
use leptos_icons::Icon;

use crate::{
    app_update::{AppUpdateSnapshot, UpdateCheckState},
    components::{Button, ButtonSize, ButtonVariant, Dialog},
    state::AppStore,
};

#[component]
pub fn UpdateNotice(store: AppStore) -> impl IntoView {
    view! {
        <div class="update-notice-region">
            <div class="sr-only" role="status" aria-live="polite" aria-atomic="true">
                {move || update_live_message(store.update.get())}
            </div>
            {move || match store.update.get() {
                UpdateCheckState::Available(snapshot) => render_update_notice(store, snapshot),
                _ => view! { <></> }.into_view(),
            }}
        </div>
    }
}

#[component]
pub fn UpdateDialog(store: AppStore) -> impl IntoView {
    view! {
        {move || {
            let open = store.update_dialog_open.get();
            let state = store.update.get();
            let update = store
                .active_update
                .get()
                .or_else(|| state.available_snapshot().cloned());

            render_update_dialog(store, open, state, update)
        }}
    }
}

fn render_update_notice(store: AppStore, snapshot: AppUpdateSnapshot) -> View {
    let target_version = snapshot.target_version_label().to_owned();
    let label = format!("Update {target_version} available");
    let description = format!(
        "Floci UI {} can update to {target_version}.",
        snapshot.current_version
    );

    view! {
        <button
            type="button"
            class="update-notice"
            aria-label=label.clone()
            on:click=move |_| store.set_update_dialog_open.set(true)
        >
            <span class="update-notice-main">
                <span class="update-notice-icon" aria-hidden="true">
                    <Icon icon=LuPackageCheck width="1em" height="1em" />
                </span>
                <span class="update-notice-copy">
                    <strong>{label}</strong>
                    <span>{description}</span>
                </span>
            </span>
            <span class="update-notice-action">
                "View notes"
                <Icon icon=LuDownload width="1em" height="1em" />
            </span>
        </button>
    }
    .into_view()
}

fn render_update_dialog(
    store: AppStore,
    open: bool,
    state: UpdateCheckState,
    update: Option<AppUpdateSnapshot>,
) -> View {
    let installing = state.is_installing();
    let error = match &state {
        UpdateCheckState::Failed(message) => Some(message.clone()),
        _ => None,
    };
    let installed = match &state {
        UpdateCheckState::Installed(message) => Some(message.clone()),
        _ => None,
    };
    let title = update.as_ref().map_or_else(
        || "App update".to_owned(),
        |snapshot| format!("Update {} available", snapshot.target_version_label()),
    );
    let description = update.as_ref().map_or_else(
        || "Update details are unavailable.".to_owned(),
        |snapshot| {
            format!(
                "Floci UI {} can update to {}.",
                snapshot.current_version,
                snapshot.target_version_label()
            )
        },
    );
    let can_install = state.can_install();
    let footer = update_dialog_footer(store, installing, can_install);

    view! {
        <Dialog
            title=title
            eyebrow="App update"
            description=description
            open=open
            loading=installing
            footer=footer
            on_close=Callback::new(move |_| {
                if !store.update.get_untracked().is_installing() {
                    store.set_update_dialog_open.set(false);
                }
            })
        >
            <div class="update-dialog-content">
                {match update {
                    Some(snapshot) => render_update_details(snapshot),
                    None => view! {
                        <p class="muted-text">
                            "No update details are available. Close this dialog and check again later."
                        </p>
                    }.into_view(),
                }}
                {installing.then(|| view! {
                    <div class="update-status" role="status">
                        <Icon icon=LuRefreshCw width="1em" height="1em" />
                        <span>"Installing update. Floci UI may restart automatically."</span>
                    </div>
                })}
                {installed.map(|message| view! {
                    <div class="update-status success" role="status">
                        <Icon icon=LuPackageCheck width="1em" height="1em" />
                        <span>{message}</span>
                    </div>
                })}
                {error.map(|message| view! {
                    <div class="dialog-error" role="alert">{message}</div>
                })}
            </div>
        </Dialog>
    }
    .into_view()
}

fn render_update_details(snapshot: AppUpdateSnapshot) -> View {
    let current_version = snapshot.current_version.clone();
    let target_version = snapshot.target_version_label().to_owned();
    let release_date = snapshot.date.clone();
    let release_notes = snapshot.release_notes_text();

    view! {
        <dl class="update-dialog-meta">
            <div>
                <dt>"Current version"</dt>
                <dd>{current_version}</dd>
            </div>
            <div>
                <dt>"Target version"</dt>
                <dd>{target_version}</dd>
            </div>
            {release_date.map(|date| view! {
                <div>
                    <dt>"Release date"</dt>
                    <dd>{date}</dd>
                </div>
            })}
        </dl>
        <section class="release-notes-section" aria-labelledby="release-notes-title">
            <h3 id="release-notes-title">"Release notes"</h3>
            <pre class="release-notes" tabindex="0">{release_notes}</pre>
        </section>
    }
    .into_view()
}

fn update_dialog_footer(store: AppStore, installing: bool, can_install: bool) -> View {
    view! {
        <div class="dialog-actions">
            <Button
                variant=ButtonVariant::Secondary
                size=ButtonSize::Medium
                disabled=installing
                on_click=Callback::new(move |_| store.set_update_dialog_open.set(false))
            >
                "Later"
            </Button>
            <Button
                variant=ButtonVariant::Primary
                size=ButtonSize::Medium
                loading=installing
                disabled=!can_install
                icon=view! {
                    <Icon icon=LuDownload width="1em" height="1em" />
                }.into_view()
                on_click=Callback::new(move |_| store.install_update())
            >
                "Update now"
            </Button>
        </div>
    }
    .into_view()
}

fn update_live_message(state: UpdateCheckState) -> String {
    match state {
        UpdateCheckState::Available(snapshot) => {
            format!("Update {} is available.", snapshot.target_version_label())
        }
        _ => String::new(),
    }
}
