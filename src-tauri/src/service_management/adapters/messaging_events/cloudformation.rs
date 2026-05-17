use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        actions::{require_name_payload, require_typed_confirmation},
        adapters::messaging_events::{
            format_timestamp, insert_attr, local_credentials, managed_inventory,
            require_resource_id, unsupported_action,
        },
        errors::ServiceManagementError,
        models::{
            ActionResult, ResourceSummary, ResourceTab, ServiceActionRequest, ServiceInventory,
        },
    },
};

#[allow(clippy::too_many_lines)]
pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let client = client(config);
    let output = client.list_stacks().send().await.map_err(|err| {
        ServiceManagementError::client_error("cloudformation", "list_stacks", err)
    })?;
    let mut resources = Vec::new();

    for stack in output.stack_summaries() {
        let Some(name) = stack.stack_name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "stack_id", stack.stack_id());
        insert_attr(
            &mut attributes,
            "template_description",
            stack.template_description(),
        );

        resources.push(ResourceSummary {
            id: format!("stack/{name}"),
            name: name.to_owned(),
            kind: "stack".to_owned(),
            status: stack
                .stack_status()
                .map_or_else(|| "unknown".to_owned(), ToString::to_string),
            created_at: format_timestamp(stack.creation_time()),
            updated_at: format_timestamp(stack.last_updated_time()),
            tags: BTreeMap::new(),
            attributes,
        });

        if let Ok(stack_detail) = client.describe_stacks().stack_name(name).send().await {
            for stack in stack_detail.stacks() {
                for parameter in stack.parameters() {
                    let Some(parameter_key) = parameter.parameter_key() else {
                        continue;
                    };
                    let mut attributes =
                        BTreeMap::from([("stack_name".to_owned(), name.to_owned())]);
                    insert_attr(&mut attributes, "value", parameter.parameter_value());
                    insert_attr(
                        &mut attributes,
                        "resolved_value",
                        parameter.resolved_value(),
                    );
                    insert_attr(
                        &mut attributes,
                        "use_previous_value",
                        parameter.use_previous_value(),
                    );

                    resources.push(ResourceSummary {
                        id: format!("parameter/{name}/{parameter_key}"),
                        name: parameter_key.to_owned(),
                        kind: "parameter".to_owned(),
                        status: "configured".to_owned(),
                        created_at: None,
                        updated_at: None,
                        tags: BTreeMap::new(),
                        attributes,
                    });
                }
            }
        }

        if let Ok(stack_resources) = client.list_stack_resources().stack_name(name).send().await {
            for resource in stack_resources.stack_resource_summaries() {
                let logical_id = resource.logical_resource_id().unwrap_or("resource");
                let mut attributes = BTreeMap::from([("stack_name".to_owned(), name.to_owned())]);
                insert_attr(
                    &mut attributes,
                    "physical_id",
                    resource.physical_resource_id(),
                );
                insert_attr(&mut attributes, "resource_type", resource.resource_type());
                insert_attr(
                    &mut attributes,
                    "resource_status_reason",
                    resource.resource_status_reason(),
                );

                resources.push(ResourceSummary {
                    id: format!("stack-resource/{name}/{logical_id}"),
                    name: logical_id.to_owned(),
                    kind: "stack-resource".to_owned(),
                    status: resource
                        .resource_status()
                        .map_or_else(|| "unknown".to_owned(), ToString::to_string),
                    created_at: None,
                    updated_at: format_timestamp(resource.last_updated_timestamp()),
                    tags: BTreeMap::new(),
                    attributes,
                });
            }
        }

        if let Ok(change_sets) = client.list_change_sets().stack_name(name).send().await {
            for change_set in change_sets.summaries() {
                let Some(change_set_name) = change_set.change_set_name() else {
                    continue;
                };
                let mut attributes = BTreeMap::from([("stack_name".to_owned(), name.to_owned())]);
                insert_attr(&mut attributes, "change_set_id", change_set.change_set_id());
                insert_attr(
                    &mut attributes,
                    "execution_status",
                    change_set.execution_status(),
                );
                insert_attr(&mut attributes, "status_reason", change_set.status_reason());

                resources.push(ResourceSummary {
                    id: format!("change-set/{name}/{change_set_name}"),
                    name: change_set_name.to_owned(),
                    kind: "change-set".to_owned(),
                    status: change_set
                        .status()
                        .map_or_else(|| "unknown".to_owned(), ToString::to_string),
                    created_at: format_timestamp(change_set.creation_time()),
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes,
                });
            }
        }

        if let Ok(events) = client.describe_stack_events().stack_name(name).send().await {
            for event in events.stack_events() {
                let event_id = event.event_id().unwrap_or("event");
                let logical_id = event.logical_resource_id().unwrap_or(name);
                let mut attributes = BTreeMap::new();
                attributes.insert("stack_name".to_owned(), name.to_owned());
                insert_attr(&mut attributes, "resource_type", event.resource_type());
                insert_attr(
                    &mut attributes,
                    "resource_status_reason",
                    event.resource_status_reason(),
                );

                resources.push(ResourceSummary {
                    id: format!("stack-event/{name}/{event_id}"),
                    name: logical_id.to_owned(),
                    kind: "stack-event".to_owned(),
                    status: event
                        .resource_status()
                        .map_or_else(|| "unknown".to_owned(), ToString::to_string),
                    created_at: format_timestamp(event.timestamp()),
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes,
                });
            }
        }
    }

    Ok(managed_inventory(
        "cloudformation",
        "CloudFormation",
        tabs(),
        resources,
        vec![
            "create_change_set".to_owned(),
            "execute_change_set".to_owned(),
            "delete_change_set".to_owned(),
            "update_stack".to_owned(),
        ],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_stack" => {
            let stack_name = require_name_payload(request, "stack_name")?;
            let template_body = request
                .payload
                .get("template_body")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(DEFAULT_TEMPLATE_BODY);
            client(config)
                .create_stack()
                .stack_name(&stack_name)
                .template_body(template_body)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("cloudformation", "create_stack", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created CloudFormation stack `{stack_name}`."),
                resource_id: Some(format!("stack/{stack_name}")),
            })
        }
        "delete_stack" => {
            let stack_name = require_typed_confirmation(request)?;
            let stack_id = require_resource_id(request, "stack/")?;
            client(config)
                .delete_stack()
                .stack_name(stack_id)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("cloudformation", "delete_stack", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted CloudFormation stack `{stack_name}`."),
                resource_id: Some(format!("stack/{stack_id}")),
            })
        }
        "refresh_stack" => {
            let stack_id = require_resource_id(request, "stack/")?;
            Ok(ActionResult {
                changed: false,
                message: "Stack refresh completed.".to_owned(),
                resource_id: Some(format!("stack/{stack_id}")),
            })
        }
        "view_stack_events" => {
            let stack_id = require_resource_id(request, "stack/")?;
            Ok(ActionResult {
                changed: false,
                message: "Stack events are loaded in the Events tab.".to_owned(),
                resource_id: Some(format!("stack/{stack_id}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "stacks".to_owned(),
            label: "Stacks".to_owned(),
            kinds: vec!["stack".to_owned()],
            empty_message: "No CloudFormation stacks were found.".to_owned(),
        },
        ResourceTab {
            key: "resources".to_owned(),
            label: "Resources".to_owned(),
            kinds: vec!["stack-resource".to_owned()],
            empty_message: "No CloudFormation stack resources are loaded.".to_owned(),
        },
        ResourceTab {
            key: "events".to_owned(),
            label: "Events".to_owned(),
            kinds: vec!["stack-event".to_owned()],
            empty_message: "No CloudFormation stack events are loaded.".to_owned(),
        },
        ResourceTab {
            key: "change-sets".to_owned(),
            label: "Change Sets".to_owned(),
            kinds: vec!["change-set".to_owned()],
            empty_message: "No CloudFormation change sets are loaded.".to_owned(),
        },
        ResourceTab {
            key: "parameters".to_owned(),
            label: "Parameters".to_owned(),
            kinds: vec!["parameter".to_owned()],
            empty_message: "No CloudFormation stack parameters are loaded.".to_owned(),
        },
    ]
}

const DEFAULT_TEMPLATE_BODY: &str = r#"{
  "AWSTemplateFormatVersion": "2010-09-09",
  "Resources": {}
}"#;

fn client(config: &AppConfig) -> aws_sdk_cloudformation::Client {
    let sdk_config = aws_sdk_cloudformation::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_cloudformation::config::Region::new(
            config.region.clone(),
        ))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_cloudformation::Client::from_conf(sdk_config)
}
