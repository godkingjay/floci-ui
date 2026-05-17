use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        actions::{require_name_payload, require_typed_confirmation},
        adapters::network_observability::{
            insert_attr, local_credentials, managed_inventory, optional_payload_i32,
            require_resource_id, unsupported_action,
        },
        errors::ServiceManagementError,
        models::{
            ActionResult, ResourceSummary, ResourceTab, ServiceActionRequest, ServiceInventory,
        },
    },
};

pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let client = client(config);
    let groups = client.describe_log_groups().send().await.map_err(|err| {
        ServiceManagementError::client_error("cloudwatchlogs", "describe_log_groups", err)
    })?;
    let mut resources = Vec::new();

    for group in groups.log_groups() {
        let Some(log_group_name) = group.log_group_name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "arn", group.arn());
        insert_attr(&mut attributes, "log_group_arn", group.log_group_arn());
        insert_attr(
            &mut attributes,
            "retention_in_days",
            group.retention_in_days(),
        );
        insert_attr(&mut attributes, "stored_bytes", group.stored_bytes());
        insert_attr(
            &mut attributes,
            "creation_time",
            group.creation_time().map(|value| format!("{value} ms")),
        );

        resources.push(ResourceSummary {
            id: format!("log-group/{log_group_name}"),
            name: log_group_name.to_owned(),
            kind: "log-group".to_owned(),
            status: group.retention_in_days().map_or_else(
                || "never expire".to_owned(),
                |days| format!("{days}d retention"),
            ),
            created_at: group.creation_time().map(|value| format!("{value} ms")),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });

        load_streams(&client, log_group_name, &mut resources).await;
        load_metric_filters(&client, log_group_name, &mut resources).await;
        load_subscription_filters(&client, log_group_name, &mut resources).await;
    }

    Ok(managed_inventory(
        "cloudwatchlogs",
        "CloudWatch Logs",
        tabs(),
        resources,
        vec!["events_require_explicit_tail".to_owned()],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_log_group" => {
            let log_group_name = require_name_payload(request, "log_group_name")?;

            client(config)
                .create_log_group()
                .log_group_name(&log_group_name)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("cloudwatchlogs", "create_log_group", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created log group `{log_group_name}`."),
                resource_id: Some(format!("log-group/{log_group_name}")),
            })
        }
        "tail_recent_events" => {
            let log_group_name = require_resource_id(request, "log-group/")?;
            let limit = optional_payload_i32(request, "limit")
                .unwrap_or(20)
                .clamp(1, 100);
            let output = client(config)
                .filter_log_events()
                .log_group_name(log_group_name)
                .limit(limit)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("cloudwatchlogs", "filter_log_events", err)
                })?;
            let latest = output
                .events()
                .iter()
                .rev()
                .filter_map(|event| event.message())
                .map(redact_log_message)
                .next()
                .unwrap_or_else(|| "No recent log events were returned.".to_owned());

            Ok(ActionResult {
                changed: false,
                message: format!(
                    "Loaded {} recent log event(s). Latest: {latest}",
                    output.events().len()
                ),
                resource_id: Some(format!("log-group/{log_group_name}")),
            })
        }
        "delete_log_group" => {
            let log_group_name = require_typed_confirmation(request)?;
            let selected_group = require_resource_id(request, "log-group/")?;

            client(config)
                .delete_log_group()
                .log_group_name(selected_group)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("cloudwatchlogs", "delete_log_group", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted log group `{log_group_name}`."),
                resource_id: Some(format!("log-group/{selected_group}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "log-groups".to_owned(),
            label: "Log Groups".to_owned(),
            kinds: vec!["log-group".to_owned()],
            empty_message: "No CloudWatch log groups were found in the local emulator.".to_owned(),
        },
        ResourceTab {
            key: "streams".to_owned(),
            label: "Streams".to_owned(),
            kinds: vec!["log-stream".to_owned()],
            empty_message: "No CloudWatch log streams are loaded.".to_owned(),
        },
        ResourceTab {
            key: "events".to_owned(),
            label: "Events".to_owned(),
            kinds: vec!["log-event".to_owned()],
            empty_message: "Choose Tail recent events on a log group to fetch log events."
                .to_owned(),
        },
        ResourceTab {
            key: "filters".to_owned(),
            label: "Filters".to_owned(),
            kinds: vec!["metric-filter".to_owned()],
            empty_message: "No CloudWatch Logs metric filters are loaded.".to_owned(),
        },
        ResourceTab {
            key: "subscriptions".to_owned(),
            label: "Subscriptions".to_owned(),
            kinds: vec!["subscription-filter".to_owned()],
            empty_message: "No CloudWatch Logs subscription filters are loaded.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_cloudwatchlogs::Client {
    let sdk_config = aws_sdk_cloudwatchlogs::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_cloudwatchlogs::config::Region::new(
            config.region.clone(),
        ))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_cloudwatchlogs::Client::from_conf(sdk_config)
}

async fn load_streams(
    client: &aws_sdk_cloudwatchlogs::Client,
    log_group_name: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client
        .describe_log_streams()
        .log_group_name(log_group_name)
        .send()
        .await
    else {
        return;
    };

    for stream in output.log_streams() {
        let Some(log_stream_name) = stream.log_stream_name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "log_group_name", Some(log_group_name));
        insert_attr(
            &mut attributes,
            "creation_time",
            stream.creation_time().map(|value| format!("{value} ms")),
        );
        insert_attr(
            &mut attributes,
            "last_event_timestamp",
            stream
                .last_event_timestamp()
                .map(|value| format!("{value} ms")),
        );
        insert_attr(
            &mut attributes,
            "last_ingestion_time",
            stream
                .last_ingestion_time()
                .map(|value| format!("{value} ms")),
        );
        resources.push(ResourceSummary {
            id: format!("log-stream/{log_group_name}/{log_stream_name}"),
            name: log_stream_name.to_owned(),
            kind: "log-stream".to_owned(),
            status: stream
                .last_event_timestamp()
                .map_or_else(|| "idle".to_owned(), |_| "active".to_owned()),
            created_at: stream.creation_time().map(|value| format!("{value} ms")),
            updated_at: stream
                .last_event_timestamp()
                .map(|value| format!("{value} ms")),
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

async fn load_metric_filters(
    client: &aws_sdk_cloudwatchlogs::Client,
    log_group_name: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client
        .describe_metric_filters()
        .log_group_name(log_group_name)
        .send()
        .await
    else {
        return;
    };

    for filter in output.metric_filters() {
        let Some(filter_name) = filter.filter_name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "log_group_name", filter.log_group_name());
        insert_attr(&mut attributes, "filter_pattern", filter.filter_pattern());
        insert_attr(
            &mut attributes,
            "metric_transformations",
            Some(filter.metric_transformations().len()),
        );

        resources.push(ResourceSummary {
            id: format!("metric-filter/{log_group_name}/{filter_name}"),
            name: filter_name.to_owned(),
            kind: "metric-filter".to_owned(),
            status: "configured".to_owned(),
            created_at: filter.creation_time().map(|value| format!("{value} ms")),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

async fn load_subscription_filters(
    client: &aws_sdk_cloudwatchlogs::Client,
    log_group_name: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client
        .describe_subscription_filters()
        .log_group_name(log_group_name)
        .send()
        .await
    else {
        return;
    };

    for filter in output.subscription_filters() {
        let Some(filter_name) = filter.filter_name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "log_group_name", filter.log_group_name());
        insert_attr(&mut attributes, "filter_pattern", filter.filter_pattern());
        insert_attr(&mut attributes, "destination_arn", filter.destination_arn());
        if filter.role_arn().is_some() {
            insert_attr(
                &mut attributes,
                "role_arn",
                Some("Sensitive role ARN redacted from inventory."),
            );
        }

        resources.push(ResourceSummary {
            id: format!("subscription-filter/{log_group_name}/{filter_name}"),
            name: filter_name.to_owned(),
            kind: "subscription-filter".to_owned(),
            status: "configured".to_owned(),
            created_at: filter.creation_time().map(|value| format!("{value} ms")),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

fn redact_log_message(message: &str) -> String {
    let lowercase = message.to_ascii_lowercase();
    if lowercase.contains("authorization")
        || lowercase.contains("x-api-key")
        || lowercase.contains("api_key")
        || lowercase.contains("apikey")
        || lowercase.contains("secret")
        || lowercase.contains("token")
    {
        return "[redacted log event]".to_owned();
    }

    const MAX_PREVIEW_CHARS: usize = 120;
    if message.len() <= MAX_PREVIEW_CHARS {
        return message.to_owned();
    }

    format!("{}...", &message[..MAX_PREVIEW_CHARS])
}
