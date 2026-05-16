use std::collections::BTreeMap;

use aws_sdk_sfn::types::StateMachineType;

use crate::{
    config::AppConfig,
    service_management::{
        actions::{require_name_payload, require_typed_confirmation},
        adapters::messaging_events::{
            format_timestamp, insert_attr, local_credentials, managed_inventory,
            optional_payload_text, require_json_payload_text, require_resource_id,
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
    let output = client.list_state_machines().send().await.map_err(|err| {
        ServiceManagementError::client_error("stepfunctions", "list_state_machines", err)
    })?;
    let mut resources = Vec::new();

    for state_machine in output.state_machines() {
        let arn = state_machine.state_machine_arn();
        let name = state_machine.name();
        let mut attributes = BTreeMap::new();
        attributes.insert("state_machine_arn".to_owned(), arn.to_owned());
        insert_attr(
            &mut attributes,
            "state_machine_type",
            Some(state_machine.r#type()),
        );

        resources.push(ResourceSummary {
            id: format!("state-machine/{arn}"),
            name: name.to_owned(),
            kind: "state-machine".to_owned(),
            status: "available".to_owned(),
            created_at: format_timestamp(Some(state_machine.creation_date())),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });

        if let Ok(description) = client
            .describe_state_machine()
            .state_machine_arn(arn)
            .send()
            .await
        {
            resources.push(ResourceSummary {
                id: format!("definition/{arn}"),
                name: format!("{name} definition"),
                kind: "definition".to_owned(),
                status: description
                    .status()
                    .map_or_else(|| "available".to_owned(), ToString::to_string),
                created_at: format_timestamp(Some(description.creation_date())),
                updated_at: None,
                tags: BTreeMap::new(),
                attributes: BTreeMap::from([
                    ("state_machine_arn".to_owned(), arn.to_owned()),
                    ("role_arn".to_owned(), description.role_arn().to_owned()),
                    ("definition".to_owned(), description.definition().to_owned()),
                ]),
            });
        }

        if let Ok(aliases) = client
            .list_state_machine_aliases()
            .state_machine_arn(arn)
            .send()
            .await
        {
            for alias in aliases.state_machine_aliases() {
                let alias_arn = alias.state_machine_alias_arn();
                resources.push(ResourceSummary {
                    id: format!("alias/{alias_arn}"),
                    name: resource_name_from_arn(alias_arn).to_owned(),
                    kind: "alias".to_owned(),
                    status: "available".to_owned(),
                    created_at: format_timestamp(Some(alias.creation_date())),
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes: BTreeMap::from([
                        ("state_machine_arn".to_owned(), arn.to_owned()),
                        ("alias_arn".to_owned(), alias_arn.to_owned()),
                    ]),
                });
            }
        }

        if let Ok(executions) = client.list_executions().state_machine_arn(arn).send().await {
            for execution in executions.executions() {
                let execution_arn = execution.execution_arn();
                let mut attributes = BTreeMap::new();
                attributes.insert("execution_arn".to_owned(), execution_arn.to_owned());
                attributes.insert("state_machine_arn".to_owned(), arn.to_owned());

                resources.push(ResourceSummary {
                    id: format!("execution/{execution_arn}"),
                    name: execution.name().to_owned(),
                    kind: "execution".to_owned(),
                    status: execution.status().to_string(),
                    created_at: format_timestamp(Some(execution.start_date())),
                    updated_at: format_timestamp(execution.stop_date()),
                    tags: BTreeMap::new(),
                    attributes,
                });
            }
        }
    }

    if let Ok(activities) = client.list_activities().send().await {
        for activity in activities.activities() {
            resources.push(ResourceSummary {
                id: format!("activity/{}", activity.activity_arn()),
                name: activity.name().to_owned(),
                kind: "activity".to_owned(),
                status: "available".to_owned(),
                created_at: format_timestamp(Some(activity.creation_date())),
                updated_at: None,
                tags: BTreeMap::new(),
                attributes: BTreeMap::from([(
                    "activity_arn".to_owned(),
                    activity.activity_arn().to_owned(),
                )]),
            });
        }
    }

    Ok(managed_inventory(
        "stepfunctions",
        "Step Functions",
        tabs(),
        resources,
        vec!["delete_activity".to_owned()],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_state_machine" => {
            let state_machine_name = require_name_payload(request, "state_machine_name")?;
            let role_arn = request
                .payload
                .get("role_arn")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("arn:aws:iam::000000000000:role/floci-stepfunctions-local");
            let definition = request
                .payload
                .get("definition")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(DEFAULT_STATE_MACHINE_DEFINITION);
            let output = client(config)
                .create_state_machine()
                .name(&state_machine_name)
                .definition(definition)
                .role_arn(role_arn)
                .r#type(StateMachineType::Standard)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error(
                        "stepfunctions",
                        "create_state_machine",
                        err,
                    )
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created Step Functions state machine `{state_machine_name}`."),
                resource_id: Some(format!("state-machine/{}", output.state_machine_arn())),
            })
        }
        "start_execution" => {
            let arn = require_resource_id(request, "state-machine/")?;
            let input = require_json_payload_text(request, "input", "{}")?;
            let mut builder = client(config)
                .start_execution()
                .state_machine_arn(arn)
                .input(input);
            if let Some(execution_name) = optional_payload_text(request, "execution_name") {
                builder = builder.name(execution_name);
            }
            let output = builder.send().await.map_err(|err| {
                ServiceManagementError::client_error("stepfunctions", "start_execution", err)
            })?;

            Ok(ActionResult {
                changed: true,
                message: format!(
                    "Started Step Functions execution `{}`.",
                    resource_name_from_arn(output.execution_arn())
                ),
                resource_id: Some(format!("execution/{}", output.execution_arn())),
            })
        }
        "stop_execution" => {
            let execution_name = require_typed_confirmation(request)?;
            let execution_arn = require_resource_id(request, "execution/")?;
            client(config)
                .stop_execution()
                .execution_arn(execution_arn)
                .cause("Stopped from floci-ui.")
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("stepfunctions", "stop_execution", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Stopped Step Functions execution `{execution_name}`."),
                resource_id: Some(format!("execution/{execution_arn}")),
            })
        }
        "delete_state_machine" => {
            let state_machine_name = require_typed_confirmation(request)?;
            let arn = require_resource_id(request, "state-machine/")?;
            client(config)
                .delete_state_machine()
                .state_machine_arn(arn)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error(
                        "stepfunctions",
                        "delete_state_machine",
                        err,
                    )
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted Step Functions state machine `{state_machine_name}`."),
                resource_id: Some(format!("state-machine/{arn}")),
            })
        }
        "refresh_state_machine" => {
            let arn = require_resource_id(request, "state-machine/")?;
            Ok(ActionResult {
                changed: false,
                message: format!(
                    "State machine `{}` refresh completed.",
                    resource_name_from_arn(arn)
                ),
                resource_id: Some(format!("state-machine/{arn}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "state-machines".to_owned(),
            label: "State Machines".to_owned(),
            kinds: vec!["state-machine".to_owned()],
            empty_message: "No Step Functions state machines were found.".to_owned(),
        },
        ResourceTab {
            key: "executions".to_owned(),
            label: "Executions".to_owned(),
            kinds: vec!["execution".to_owned()],
            empty_message: "No Step Functions executions are loaded.".to_owned(),
        },
        ResourceTab {
            key: "activities".to_owned(),
            label: "Activities".to_owned(),
            kinds: vec!["activity".to_owned()],
            empty_message: "No Step Functions activities are loaded.".to_owned(),
        },
        ResourceTab {
            key: "definitions".to_owned(),
            label: "Definitions".to_owned(),
            kinds: vec!["definition".to_owned()],
            empty_message: "No Step Functions definitions are loaded.".to_owned(),
        },
        ResourceTab {
            key: "aliases".to_owned(),
            label: "Aliases".to_owned(),
            kinds: vec!["alias".to_owned()],
            empty_message: "No Step Functions aliases are loaded.".to_owned(),
        },
    ]
}

const DEFAULT_STATE_MACHINE_DEFINITION: &str = r#"{
  "Comment": "Floci UI local placeholder",
  "StartAt": "Pass",
  "States": {
    "Pass": {
      "Type": "Pass",
      "End": true
    }
  }
}"#;

fn client(config: &AppConfig) -> aws_sdk_sfn::Client {
    let sdk_config = aws_sdk_sfn::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_sfn::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_sfn::Client::from_conf(sdk_config)
}
