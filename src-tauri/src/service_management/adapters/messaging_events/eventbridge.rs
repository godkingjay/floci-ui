use std::collections::BTreeMap;

use aws_sdk_eventbridge::types::{PutEventsRequestEntry, RuleState};

use crate::{
    config::AppConfig,
    service_management::{
        actions::{require_name_payload, require_typed_confirmation},
        adapters::messaging_events::{
            format_timestamp, insert_attr, local_credentials, managed_inventory,
            require_json_payload_text, require_payload_text, require_resource_id,
            resource_name_from_arn, unsupported_action,
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
    let mut resources = Vec::new();
    let mut bus_names = Vec::new();

    let buses = client.list_event_buses().send().await.map_err(|err| {
        ServiceManagementError::client_error("eventbridge", "list_event_buses", err)
    })?;
    for bus in buses.event_buses() {
        let Some(name) = bus.name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "arn", bus.arn());
        insert_attr(&mut attributes, "policy", bus.policy());
        bus_names.push(name.to_owned());

        resources.push(ResourceSummary {
            id: format!("event-bus/{name}"),
            name: name.to_owned(),
            kind: "event-bus".to_owned(),
            status: "available".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    for bus_name in &bus_names {
        let Ok(rules) = client.list_rules().event_bus_name(bus_name).send().await else {
            continue;
        };
        for rule in rules.rules() {
            let Some(name) = rule.name() else {
                continue;
            };
            let mut attributes = BTreeMap::new();
            insert_attr(&mut attributes, "arn", rule.arn());
            attributes.insert("event_bus_name".to_owned(), bus_name.clone());
            insert_attr(
                &mut attributes,
                "schedule_expression",
                rule.schedule_expression(),
            );
            insert_attr(&mut attributes, "event_pattern", rule.event_pattern());

            resources.push(ResourceSummary {
                id: format!("rule/{bus_name}/{name}"),
                name: name.to_owned(),
                kind: "rule".to_owned(),
                status: rule
                    .state()
                    .map_or_else(|| "unknown".to_owned(), ToString::to_string),
                created_at: None,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes,
            });

            if let Ok(targets) = client
                .list_targets_by_rule()
                .event_bus_name(bus_name)
                .rule(name)
                .send()
                .await
            {
                for target in targets.targets() {
                    let mut attributes = BTreeMap::from([
                        ("event_bus_name".to_owned(), bus_name.clone()),
                        ("rule_name".to_owned(), name.to_owned()),
                        ("target_arn".to_owned(), target.arn().to_owned()),
                    ]);
                    insert_attr(&mut attributes, "role_arn", target.role_arn());
                    insert_attr(&mut attributes, "input_path", target.input_path());

                    resources.push(ResourceSummary {
                        id: format!("target/{bus_name}/{name}/{}", target.id()),
                        name: target.id().to_owned(),
                        kind: "target".to_owned(),
                        status: "attached".to_owned(),
                        created_at: None,
                        updated_at: None,
                        tags: BTreeMap::new(),
                        attributes,
                    });
                }
            }
        }
    }

    if let Ok(archives) = client.list_archives().send().await {
        for archive in archives.archives() {
            let Some(name) = archive.archive_name() else {
                continue;
            };
            let mut attributes = BTreeMap::new();
            insert_attr(
                &mut attributes,
                "event_source_arn",
                archive.event_source_arn(),
            );
            insert_attr(&mut attributes, "state_reason", archive.state_reason());
            insert_attr(&mut attributes, "retention_days", archive.retention_days());
            insert_attr(&mut attributes, "size_bytes", Some(archive.size_bytes()));
            insert_attr(&mut attributes, "event_count", Some(archive.event_count()));

            resources.push(ResourceSummary {
                id: format!("archive/{name}"),
                name: name.to_owned(),
                kind: "archive".to_owned(),
                status: archive
                    .state()
                    .map_or_else(|| "unknown".to_owned(), ToString::to_string),
                created_at: format_timestamp(archive.creation_time()),
                updated_at: None,
                tags: BTreeMap::new(),
                attributes,
            });
        }
    }

    if let Ok(replays) = client.list_replays().send().await {
        for replay in replays.replays() {
            let Some(name) = replay.replay_name() else {
                continue;
            };
            let mut attributes = BTreeMap::new();
            insert_attr(
                &mut attributes,
                "event_source_arn",
                replay.event_source_arn(),
            );
            insert_attr(&mut attributes, "state_reason", replay.state_reason());
            insert_attr(
                &mut attributes,
                "last_replayed_at",
                format_timestamp(replay.event_last_replayed_time()),
            );

            resources.push(ResourceSummary {
                id: format!("replay/{name}"),
                name: resource_name_from_arn(name).to_owned(),
                kind: "replay".to_owned(),
                status: replay
                    .state()
                    .map_or_else(|| "unknown".to_owned(), ToString::to_string),
                created_at: format_timestamp(replay.replay_start_time()),
                updated_at: format_timestamp(replay.replay_end_time()),
                tags: BTreeMap::new(),
                attributes,
            });
        }
    }

    Ok(managed_inventory(
        "eventbridge",
        "EventBridge",
        tabs(),
        resources,
        vec!["put_target".to_owned(), "delete_target".to_owned()],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_event_bus" => {
            let bus_name = require_name_payload(request, "event_bus_name")?;
            client(config)
                .create_event_bus()
                .name(&bus_name)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("eventbridge", "create_event_bus", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created EventBridge bus `{bus_name}`."),
                resource_id: Some(format!("event-bus/{bus_name}")),
            })
        }
        "delete_event_bus" => {
            let bus_name = require_typed_confirmation(request)?;
            let bus_id = require_resource_id(request, "event-bus/")?;
            client(config)
                .delete_event_bus()
                .name(bus_id)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("eventbridge", "delete_event_bus", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted EventBridge bus `{bus_name}`."),
                resource_id: Some(format!("event-bus/{bus_id}")),
            })
        }
        "create_rule" => {
            let bus_name = require_resource_id(request, "event-bus/")?;
            let rule_name = require_payload_text(request, "rule_name")?;
            let event_pattern =
                require_json_payload_text(request, "event_pattern", DEFAULT_EVENT_PATTERN)?;
            let output = client(config)
                .put_rule()
                .event_bus_name(bus_name)
                .name(&rule_name)
                .event_pattern(event_pattern)
                .state(RuleState::Enabled)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("eventbridge", "create_rule", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created EventBridge rule `{rule_name}`."),
                resource_id: output
                    .rule_arn()
                    .map(|_| format!("rule/{bus_name}/{rule_name}")),
            })
        }
        "delete_rule" => {
            let rule_name = require_typed_confirmation(request)?;
            let (bus_name, rule_id) = require_rule_id(request)?;
            client(config)
                .delete_rule()
                .event_bus_name(bus_name)
                .name(rule_id)
                .force(true)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("eventbridge", "delete_rule", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted EventBridge rule `{rule_name}`."),
                resource_id: Some(format!("rule/{bus_name}/{rule_id}")),
            })
        }
        "put_event" => {
            let bus_name = require_resource_id(request, "event-bus/")?;
            let source = require_payload_text(request, "source")?;
            let detail_type = require_payload_text(request, "detail_type")?;
            let detail = require_json_payload_text(request, "detail", "{}")?;
            let entry = PutEventsRequestEntry::builder()
                .event_bus_name(bus_name)
                .source(source)
                .detail_type(detail_type)
                .detail(detail)
                .build();
            let output = client(config)
                .put_events()
                .entries(entry)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("eventbridge", "put_event", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!(
                    "Put EventBridge event with {} failed entries.",
                    output.failed_entry_count()
                ),
                resource_id: Some(format!("event-bus/{bus_name}")),
            })
        }
        "refresh_event_bus" => {
            let bus_name = require_resource_id(request, "event-bus/")?;
            Ok(ActionResult {
                changed: false,
                message: "Event bus refresh completed.".to_owned(),
                resource_id: Some(format!("event-bus/{bus_name}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "event-buses".to_owned(),
            label: "Buses".to_owned(),
            kinds: vec!["event-bus".to_owned()],
            empty_message: "No EventBridge event buses were found.".to_owned(),
        },
        ResourceTab {
            key: "rules".to_owned(),
            label: "Rules".to_owned(),
            kinds: vec!["rule".to_owned()],
            empty_message: "No EventBridge rules are loaded.".to_owned(),
        },
        ResourceTab {
            key: "targets".to_owned(),
            label: "Targets".to_owned(),
            kinds: vec!["target".to_owned()],
            empty_message: "No EventBridge targets are loaded.".to_owned(),
        },
        ResourceTab {
            key: "archives".to_owned(),
            label: "Archives".to_owned(),
            kinds: vec!["archive".to_owned()],
            empty_message: "No EventBridge archives are loaded.".to_owned(),
        },
        ResourceTab {
            key: "replays".to_owned(),
            label: "Replays".to_owned(),
            kinds: vec!["replay".to_owned()],
            empty_message: "No EventBridge replays are loaded.".to_owned(),
        },
    ]
}

const DEFAULT_EVENT_PATTERN: &str = r#"{"source":["floci.ui"]}"#;

fn require_rule_id(request: &ServiceActionRequest) -> Result<(&str, &str), ServiceManagementError> {
    let value = require_resource_id(request, "rule/")?;
    let Some((bus_name, rule_name)) = value.split_once('/') else {
        return Err(ServiceManagementError::invalid_input(
            request.service_key.clone(),
            request.action.clone(),
            "Rule resources must include both event bus and rule name.",
        ));
    };

    Ok((bus_name, rule_name))
}

fn client(config: &AppConfig) -> aws_sdk_eventbridge::Client {
    let sdk_config = aws_sdk_eventbridge::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_eventbridge::config::Region::new(
            config.region.clone(),
        ))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_eventbridge::Client::from_conf(sdk_config)
}
