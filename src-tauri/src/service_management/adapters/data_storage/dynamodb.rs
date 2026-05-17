use std::collections::BTreeMap;

use aws_credential_types::Credentials;
use aws_sdk_dynamodb::types::{
    AttributeDefinition, BillingMode, KeySchemaElement, KeyType, ScalarAttributeType,
};

use crate::{
    config::AppConfig,
    service_management::{
        actions::{require_name_payload, require_typed_confirmation},
        errors::ServiceManagementError,
        models::{
            ActionResult, ResourceSummary, ResourceTab, ServiceActionRequest, ServiceInventory,
            ServiceSupportLevel,
        },
    },
};

#[allow(clippy::too_many_lines)]
pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let client = client(config);
    let output = client
        .list_tables()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("dynamodb", "list_tables", err))?;

    let mut resources = Vec::new();

    for table_name in output.table_names() {
        let detail = client
            .describe_table()
            .table_name(table_name)
            .send()
            .await
            .map_err(|err| {
                ServiceManagementError::client_error("dynamodb", "describe_table", err)
            })?;
        let Some(table) = detail.table() else {
            continue;
        };
        let status = table
            .table_status()
            .map_or_else(|| "unknown".to_owned(), ToString::to_string);

        resources.push(ResourceSummary {
            id: format!("table/{table_name}"),
            name: table_name.clone(),
            kind: "table".to_owned(),
            status,
            created_at: table.creation_date_time().map(|date| format!("{date:?}")),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes: BTreeMap::from([
                (
                    "item_count".to_owned(),
                    table.item_count().unwrap_or_default().to_string(),
                ),
                (
                    "table_size_bytes".to_owned(),
                    table.table_size_bytes().unwrap_or_default().to_string(),
                ),
            ]),
        });

        for index in table.global_secondary_indexes() {
            if let Some(index_name) = index.index_name() {
                resources.push(ResourceSummary {
                    id: format!("gsi/{table_name}/{index_name}"),
                    name: index_name.to_owned(),
                    kind: "global-secondary-index".to_owned(),
                    status: index
                        .index_status()
                        .map_or_else(|| "unknown".to_owned(), ToString::to_string),
                    created_at: None,
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes: BTreeMap::from([("table".to_owned(), table_name.clone())]),
                });
            }
        }

        for index in table.local_secondary_indexes() {
            if let Some(index_name) = index.index_name() {
                resources.push(ResourceSummary {
                    id: format!("lsi/{table_name}/{index_name}"),
                    name: index_name.to_owned(),
                    kind: "local-secondary-index".to_owned(),
                    status: "attached".to_owned(),
                    created_at: None,
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes: BTreeMap::from([("table".to_owned(), table_name.clone())]),
                });
            }
        }
    }

    Ok(ServiceInventory {
        service_key: "dynamodb".to_owned(),
        service_label: "DynamoDB".to_owned(),
        support_level: ServiceSupportLevel::Managed,
        refreshed_at: crate::service_management::actions::now_rfc3339(),
        tabs: tabs(),
        resources,
        unsupported_operations: vec!["toggle_ttl".to_owned()],
    })
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_table" => {
            let table_name = require_name_payload(request, "table_name")?;
            let partition_key = request
                .payload
                .get("partition_key")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("id");
            let attribute = AttributeDefinition::builder()
                .attribute_name(partition_key)
                .attribute_type(ScalarAttributeType::S)
                .build()
                .map_err(|err| {
                    ServiceManagementError::invalid_input(
                        "dynamodb",
                        "create_table",
                        err.to_string(),
                    )
                })?;
            let key_schema = KeySchemaElement::builder()
                .attribute_name(partition_key)
                .key_type(KeyType::Hash)
                .build()
                .map_err(|err| {
                    ServiceManagementError::invalid_input(
                        "dynamodb",
                        "create_table",
                        err.to_string(),
                    )
                })?;

            client(config)
                .create_table()
                .table_name(&table_name)
                .attribute_definitions(attribute)
                .key_schema(key_schema)
                .billing_mode(BillingMode::PayPerRequest)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("dynamodb", "create_table", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created DynamoDB table `{table_name}`."),
                resource_id: Some(format!("table/{table_name}")),
            })
        }
        "delete_table" => {
            let table_name = require_typed_confirmation(request)?;
            client(config)
                .delete_table()
                .table_name(&table_name)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("dynamodb", "delete_table", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted DynamoDB table `{table_name}`."),
                resource_id: Some(format!("table/{table_name}")),
            })
        }
        "refresh_table_metadata" => Ok(ActionResult {
            changed: false,
            message: "Table metadata refresh completed.".to_owned(),
            resource_id: request.resource_id.clone(),
        }),
        "toggle_ttl" => Err(ServiceManagementError::unsupported_operation(
            "dynamodb",
            "toggle_ttl",
            "TTL changes are disabled until Floci confirms UpdateTimeToLive support.",
        )),
        action => Err(ServiceManagementError::unsupported_operation(
            "dynamodb",
            action,
            format!("DynamoDB action `{action}` is not supported."),
        )),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "tables".to_owned(),
            label: "Tables".to_owned(),
            kinds: vec!["table".to_owned()],
            empty_message: "No DynamoDB tables were found in the local emulator.".to_owned(),
        },
        ResourceTab {
            key: "indexes".to_owned(),
            label: "Indexes".to_owned(),
            kinds: vec![
                "global-secondary-index".to_owned(),
                "local-secondary-index".to_owned(),
            ],
            empty_message: "No DynamoDB secondary indexes are attached.".to_owned(),
        },
        ResourceTab {
            key: "streams".to_owned(),
            label: "Streams".to_owned(),
            kinds: vec!["stream".to_owned()],
            empty_message: "No DynamoDB streams are enabled.".to_owned(),
        },
        ResourceTab {
            key: "ttl".to_owned(),
            label: "TTL".to_owned(),
            kinds: vec!["ttl".to_owned()],
            empty_message: "No TTL metadata is loaded.".to_owned(),
        },
        ResourceTab {
            key: "backups".to_owned(),
            label: "Backups".to_owned(),
            kinds: vec!["backup".to_owned()],
            empty_message: "No DynamoDB backups are loaded.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_dynamodb::Client {
    let sdk_config = aws_sdk_dynamodb::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_dynamodb::config::Region::new(config.region.clone()))
        .credentials_provider(Credentials::new(
            config.access_key_id.clone(),
            config.secret_access_key.clone(),
            None,
            None,
            "floci-ui",
        ))
        .build();

    aws_sdk_dynamodb::Client::from_conf(sdk_config)
}
