#![allow(
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::wildcard_imports
)]

use icondata::Icon as IconData;
use leptos::*;
use leptos_icons::Icon;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColumnSpec {
    pub key: String,
    pub label: String,
    pub icon: Option<IconData>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SortState {
    pub column_key: String,
    pub direction: SortDirection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RowActionSpec {
    pub label: String,
    pub disabled: bool,
    pub icon: Option<IconData>,
}

#[component]
pub fn DataTable(
    columns: Vec<ColumnSpec>,
    rows: Vec<Vec<String>>,
    #[prop(optional)] loading: bool,
    #[prop(optional, into)] empty_message: String,
    #[prop(optional)] row_actions: Vec<RowActionSpec>,
) -> impl IntoView {
    let column_count = columns.len() + usize::from(!row_actions.is_empty());
    let empty_text = if empty_message.is_empty() {
        "No rows available.".to_owned()
    } else {
        empty_message
    };

    view! {
        <div class="data-table-wrap">
            <table class="data-table">
                <thead>
                    <tr>
                        {columns.iter().map(|column| view! {
                            <th scope="col">
                                <span class="table-header-label">
                                    {column.icon.map(|icon| view! {
                                        <Icon icon=icon width="1em" height="1em" />
                                    })}
                                    {column.label.clone()}
                                </span>
                            </th>
                        }).collect_view()}
                        {(!row_actions.is_empty()).then(|| view! { <th scope="col">"Actions"</th> })}
                    </tr>
                </thead>
                <tbody>
                    {if loading {
                        view! {
                            <tr class="table-skeleton-row">
                                <td colspan=column_count>"Loading resources..."</td>
                            </tr>
                        }.into_view()
                    } else if rows.is_empty() {
                        view! {
                            <tr>
                                <td colspan=column_count class="table-empty-cell">{empty_text}</td>
                            </tr>
                        }.into_view()
                    } else {
                        rows.into_iter().map(|row| {
                            let actions = row_actions.clone();
                            view! {
                                <tr>
                                    {row.into_iter().map(|cell| view! { <td>{cell}</td> }).collect_view()}
                                    {(!actions.is_empty()).then(|| view! {
                                        <td class="table-actions">
                                            {actions.into_iter().map(|action| view! {
                                                <button
                                                    type="button"
                                                    class="table-action-button"
                                                    disabled=action.disabled
                                                >
                                                    {action.icon.map(|icon| view! {
                                                        <Icon icon=icon width="1em" height="1em" />
                                                    })}
                                                    {action.label}
                                                </button>
                                            }).collect_view()}
                                        </td>
                                    })}
                                </tr>
                            }
                        }).collect_view()
                    }}
                </tbody>
            </table>
        </div>
    }
}
