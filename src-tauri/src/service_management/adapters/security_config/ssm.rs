use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        adapters::security_config::{
            format_timestamp, insert_attr, local_credentials, managed_inventory,
            redacted_value_marker, require_resource_id, unsupported_action,
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
    let parameters =
        client.describe_parameters().send().await.map_err(|err| {
            ServiceManagementError::client_error("ssm", "describe_parameters", err)
        })?;
    let mut resources = Vec::new();

    for parameter in parameters.parameters() {
        let Some(name) = parameter.name() else {
            continue;
        };
        let parameter_type = parameter
            .r#type()
            .map_or_else(|| "Unknown".to_owned(), ToString::to_string);
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "arn", parameter.arn());
        insert_attr(&mut attributes, "type", Some(parameter_type.clone()));
        insert_attr(&mut attributes, "key_id", parameter.key_id());
        insert_attr(&mut attributes, "description", parameter.description());
        insert_attr(&mut attributes, "version", Some(parameter.version()));
        insert_attr(
            &mut attributes,
            "tier",
            parameter.tier().map(ToString::to_string),
        );
        insert_attr(&mut attributes, "data_type", parameter.data_type());
        attributes.insert(
            "value".to_owned(),
            if parameter_type == "SecureString" {
                redacted_value_marker()
            } else {
                "Hidden by default; fetch through explicit action only.".to_owned()
            },
        );

        resources.push(ResourceSummary {
            id: format!("parameter/{name}"),
            name: name.to_owned(),
            kind: "parameter".to_owned(),
            status: parameter_type,
            created_at: None,
            updated_at: format_timestamp(parameter.last_modified_date()),
            tags: BTreeMap::new(),
            attributes,
        });
    }

    if let Ok(documents) = client.list_documents().send().await {
        for document in documents.document_identifiers() {
            let Some(name) = document.name() else {
                continue;
            };
            let mut attributes = BTreeMap::new();
            insert_attr(&mut attributes, "owner", document.owner());
            insert_attr(
                &mut attributes,
                "document_type",
                document.document_type().map(ToString::to_string),
            );
            insert_attr(
                &mut attributes,
                "document_format",
                document.document_format().map(ToString::to_string),
            );
            insert_attr(&mut attributes, "schema_version", document.schema_version());
            insert_attr(
                &mut attributes,
                "document_version",
                document.document_version(),
            );
            insert_attr(&mut attributes, "target_type", document.target_type());

            resources.push(ResourceSummary {
                id: format!("document/{name}"),
                name: document.display_name().unwrap_or(name).to_owned(),
                kind: "document".to_owned(),
                status: document
                    .review_status()
                    .map_or_else(|| "available".to_owned(), ToString::to_string),
                created_at: format_timestamp(document.created_date()),
                updated_at: None,
                tags: BTreeMap::new(),
                attributes,
            });
        }
    }

    if let Ok(invocations) = client.list_command_invocations().send().await {
        for invocation in invocations.command_invocations() {
            let Some(command_id) = invocation.command_id() else {
                continue;
            };
            let instance_id = invocation.instance_id().unwrap_or("unknown");
            resources.push(ResourceSummary {
                id: format!("command/{command_id}/{instance_id}"),
                name: command_id.to_owned(),
                kind: "command".to_owned(),
                status: invocation
                    .status()
                    .map_or_else(|| "unknown".to_owned(), ToString::to_string),
                created_at: format_timestamp(invocation.requested_date_time()),
                updated_at: None,
                tags: BTreeMap::new(),
                attributes: BTreeMap::from([
                    ("instance_id".to_owned(), instance_id.to_owned()),
                    (
                        "document_name".to_owned(),
                        invocation.document_name().unwrap_or("unknown").to_owned(),
                    ),
                    (
                        "status_details".to_owned(),
                        invocation.status_details().unwrap_or("unknown").to_owned(),
                    ),
                ]),
            });
        }
    }

    if let Ok(instances) = client.describe_instance_information().send().await {
        for instance in instances.instance_information_list() {
            let Some(instance_id) = instance.instance_id() else {
                continue;
            };
            let mut attributes = BTreeMap::new();
            insert_attr(&mut attributes, "agent_version", instance.agent_version());
            insert_attr(&mut attributes, "platform_name", instance.platform_name());
            insert_attr(
                &mut attributes,
                "platform_version",
                instance.platform_version(),
            );
            insert_attr(
                &mut attributes,
                "platform_type",
                instance.platform_type().map(ToString::to_string),
            );
            insert_attr(&mut attributes, "iam_role", instance.iam_role());

            resources.push(ResourceSummary {
                id: format!("managed-instance/{instance_id}"),
                name: instance.name().unwrap_or(instance_id).to_owned(),
                kind: "managed-instance".to_owned(),
                status: instance
                    .ping_status()
                    .map_or_else(|| "unknown".to_owned(), ToString::to_string),
                created_at: format_timestamp(instance.registration_date()),
                updated_at: format_timestamp(instance.last_ping_date_time()),
                tags: BTreeMap::new(),
                attributes,
            });
        }
    }

    Ok(managed_inventory(
        "ssm",
        "SSM",
        tabs(),
        resources,
        vec![
            "put_parameter".to_owned(),
            "delete_parameter".to_owned(),
            "send_command".to_owned(),
        ],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "reveal_parameter_value" => {
            let parameter_name = require_resource_id(request, "parameter/")?;
            let output = client(config)
                .get_parameter()
                .name(parameter_name)
                .with_decryption(true)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("ssm", "get_parameter", err))?;
            let parameter = output.parameter();
            let value_length = parameter
                .and_then(|parameter| parameter.value())
                .map(str::len)
                .unwrap_or(0);
            let value_type = parameter
                .and_then(|parameter| parameter.r#type())
                .map_or_else(|| "Unknown".to_owned(), ToString::to_string);

            Ok(ActionResult {
                changed: false,
                message: format!(
                    "SSM parameter value was fetched for `{parameter_name}` and immediately cleared from the UI action state. Type: {value_type}; length: {value_length} bytes. The value is not stored in inventory, logs, or screenshots."
                ),
                resource_id: Some(format!("parameter/{parameter_name}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        tab(
            "parameters",
            "Parameters",
            &["parameter"],
            "No SSM parameters were found.",
        ),
        tab(
            "documents",
            "Documents",
            &["document"],
            "No SSM documents were found.",
        ),
        tab(
            "commands",
            "Command Invocations",
            &["command"],
            "No SSM command invocations were found.",
        ),
        tab(
            "managed-instances",
            "Managed Instances",
            &["managed-instance"],
            "No SSM managed instances were found.",
        ),
    ]
}

fn tab(key: &str, label: &str, kinds: &[&str], empty_message: &str) -> ResourceTab {
    ResourceTab {
        key: key.to_owned(),
        label: label.to_owned(),
        kinds: kinds.iter().map(|kind| (*kind).to_owned()).collect(),
        empty_message: empty_message.to_owned(),
    }
}

fn client(config: &AppConfig) -> aws_sdk_ssm::Client {
    let sdk_config = aws_sdk_ssm::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_ssm::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_ssm::Client::from_conf(sdk_config)
}
