use std::collections::BTreeMap;

use aws_sdk_lambda::{
    primitives::Blob,
    types::{FunctionCode, Runtime},
};

use crate::{
    config::AppConfig,
    service_management::{
        actions::{require_name_payload, require_typed_confirmation},
        adapters::compute_build::{
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

const DEFAULT_HANDLER: &str = "bootstrap";
const DEFAULT_ROLE_ARN: &str = "arn:aws:iam::000000000000:role/floci-lambda-local";

pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let client = client(config);
    let output = client
        .list_functions()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("lambda", "list_functions", err))?;
    let mut resources = Vec::new();

    for function in output.functions() {
        let Some(function_name) = function.function_name() else {
            continue;
        };
        let function_arn = function.function_arn().unwrap_or(function_name).to_owned();
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "function_arn", Some(function_arn.clone()));
        insert_attr(&mut attributes, "runtime", function.runtime());
        insert_attr(&mut attributes, "handler", function.handler());
        insert_attr(&mut attributes, "version", function.version());
        insert_attr(&mut attributes, "code_size", Some(function.code_size()));
        insert_attr(&mut attributes, "memory_size", function.memory_size());
        insert_attr(&mut attributes, "timeout", function.timeout());

        resources.push(ResourceSummary {
            id: format!("function/{function_name}"),
            name: function_name.to_owned(),
            kind: "function".to_owned(),
            status: function
                .state()
                .map(ToString::to_string)
                .unwrap_or_else(|| "available".to_owned()),
            created_at: function.last_modified().map(ToOwned::to_owned),
            updated_at: function.last_modified().map(ToOwned::to_owned),
            tags: BTreeMap::new(),
            attributes: attributes.clone(),
        });

        resources.push(ResourceSummary {
            id: format!("configuration/{function_name}"),
            name: format!("{function_name} configuration"),
            kind: "configuration".to_owned(),
            status: "loaded".to_owned(),
            created_at: function.last_modified().map(ToOwned::to_owned),
            updated_at: function.last_modified().map(ToOwned::to_owned),
            tags: BTreeMap::new(),
            attributes,
        });

        if let Ok(aliases) = client
            .list_aliases()
            .function_name(function_name)
            .send()
            .await
        {
            for alias in aliases.aliases() {
                let Some(alias_name) = alias.name() else {
                    continue;
                };
                let mut attributes =
                    BTreeMap::from([("function_name".to_owned(), function_name.to_owned())]);
                insert_attr(&mut attributes, "alias_arn", alias.alias_arn());
                insert_attr(
                    &mut attributes,
                    "function_version",
                    alias.function_version(),
                );

                resources.push(ResourceSummary {
                    id: format!("alias/{function_name}/{alias_name}"),
                    name: alias_name.to_owned(),
                    kind: "alias".to_owned(),
                    status: "available".to_owned(),
                    created_at: None,
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes,
                });
            }
        }

        if let Ok(versions) = client
            .list_versions_by_function()
            .function_name(function_name)
            .send()
            .await
        {
            for version in versions.versions() {
                let Some(version_name) = version.version() else {
                    continue;
                };
                if version_name == "$LATEST" {
                    continue;
                }

                resources.push(ResourceSummary {
                    id: format!("version/{function_name}/{version_name}"),
                    name: version_name.to_owned(),
                    kind: "version".to_owned(),
                    status: version
                        .state()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "available".to_owned()),
                    created_at: version.last_modified().map(ToOwned::to_owned),
                    updated_at: version.last_modified().map(ToOwned::to_owned),
                    tags: BTreeMap::new(),
                    attributes: BTreeMap::from([
                        ("function_name".to_owned(), function_name.to_owned()),
                        ("function_arn".to_owned(), function_arn.clone()),
                    ]),
                });
            }
        }
    }

    if let Ok(mappings) = client.list_event_source_mappings().send().await {
        for mapping in mappings.event_source_mappings() {
            let Some(uuid) = mapping.uuid() else {
                continue;
            };
            let mut attributes = BTreeMap::new();
            insert_attr(&mut attributes, "function_arn", mapping.function_arn());
            insert_attr(
                &mut attributes,
                "event_source_arn",
                mapping.event_source_arn(),
            );
            insert_attr(&mut attributes, "batch_size", mapping.batch_size());

            resources.push(ResourceSummary {
                id: format!("event-source-mapping/{uuid}"),
                name: uuid.to_owned(),
                kind: "event-source-mapping".to_owned(),
                status: mapping
                    .state()
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| "enabled".to_owned()),
                created_at: format_timestamp(mapping.last_modified()),
                updated_at: format_timestamp(mapping.last_modified()),
                tags: BTreeMap::new(),
                attributes,
            });
        }
    }

    Ok(managed_inventory(
        "lambda",
        "Lambda",
        tabs(),
        resources,
        vec![
            "update_function_code".to_owned(),
            "publish_version".to_owned(),
            "create_alias".to_owned(),
            "delete_alias".to_owned(),
        ],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_function" => {
            let function_name = require_name_payload(request, "function_name")?;
            let role_arn = optional_payload_text(request, "role_arn")
                .unwrap_or_else(|| DEFAULT_ROLE_ARN.to_owned());
            let handler = optional_payload_text(request, "handler")
                .unwrap_or_else(|| DEFAULT_HANDLER.to_owned());

            client(config)
                .create_function()
                .function_name(&function_name)
                .role(role_arn)
                .handler(handler)
                .runtime(Runtime::Providedal2023)
                .code(
                    FunctionCode::builder()
                        .zip_file(Blob::new(Vec::new()))
                        .build(),
                )
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("lambda", "create_function", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created Lambda function metadata for `{function_name}`."),
                resource_id: Some(format!("function/{function_name}")),
            })
        }
        "delete_function" => {
            let function_name = require_typed_confirmation(request)?;
            client(config)
                .delete_function()
                .function_name(&function_name)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("lambda", "delete_function", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted Lambda function `{function_name}`."),
                resource_id: Some(format!("function/{function_name}")),
            })
        }
        "invoke_function" => {
            let function_name = require_resource_id(request, "function/")?;
            let payload = require_json_payload_text(request, "payload", "{}")?;
            let output = client(config)
                .invoke()
                .function_name(function_name)
                .payload(Blob::new(payload.into_bytes()))
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("lambda", "invoke", err))?;
            let response = output
                .payload()
                .map(|payload| String::from_utf8_lossy(payload.as_ref()).to_string())
                .filter(|payload| !payload.is_empty())
                .unwrap_or_else(|| "{}".to_owned());
            let function_error = output.function_error().unwrap_or("none");

            Ok(ActionResult {
                changed: false,
                message: format!(
                    "Invoked Lambda `{}` with status {} and function error `{}`. Response preview: {}",
                    resource_name_from_arn(function_name),
                    output.status_code(),
                    function_error,
                    response.chars().take(240).collect::<String>(),
                ),
                resource_id: Some(format!("function/{function_name}")),
            })
        }
        "refresh_function_configuration" => {
            let function_name = require_resource_id(request, "function/")?;
            Ok(ActionResult {
                changed: false,
                message: format!("Refreshed Lambda configuration for `{function_name}`."),
                resource_id: Some(format!("function/{function_name}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "functions".to_owned(),
            label: "Functions".to_owned(),
            kinds: vec!["function".to_owned()],
            empty_message: "No Lambda functions were found in the local emulator.".to_owned(),
        },
        ResourceTab {
            key: "versions".to_owned(),
            label: "Versions".to_owned(),
            kinds: vec!["version".to_owned()],
            empty_message: "No Lambda published versions are loaded.".to_owned(),
        },
        ResourceTab {
            key: "aliases".to_owned(),
            label: "Aliases".to_owned(),
            kinds: vec!["alias".to_owned()],
            empty_message: "No Lambda aliases are loaded.".to_owned(),
        },
        ResourceTab {
            key: "event-sources".to_owned(),
            label: "Event Sources".to_owned(),
            kinds: vec!["event-source-mapping".to_owned()],
            empty_message: "No Lambda event source mappings are loaded.".to_owned(),
        },
        ResourceTab {
            key: "environment".to_owned(),
            label: "Environment".to_owned(),
            kinds: vec!["configuration".to_owned()],
            empty_message: "Configuration summaries exclude environment variable values."
                .to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_lambda::Client {
    let sdk_config = aws_sdk_lambda::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_lambda::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_lambda::Client::from_conf(sdk_config)
}
