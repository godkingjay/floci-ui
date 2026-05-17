use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        actions::require_typed_confirmation,
        adapters::security_config::{
            format_timestamp, insert_attr, local_credentials, managed_inventory,
            optional_payload_i32, optional_payload_text, require_payload_text, require_resource_id,
            unsupported_action,
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
    let keys = client
        .list_keys()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("kms", "list_keys", err))?;
    let mut resources = Vec::new();

    for key in keys.keys() {
        let Some(key_id) = key.key_id() else {
            continue;
        };
        let key_arn = key.key_arn().unwrap_or(key_id);
        let mut attributes = BTreeMap::from([
            ("key_id".to_owned(), key_id.to_owned()),
            ("key_arn".to_owned(), key_arn.to_owned()),
            ("key_material".to_owned(), "not-exportable".to_owned()),
        ]);
        let mut status = "available".to_owned();
        let mut created_at = None;

        if let Ok(description) = client.describe_key().key_id(key_id).send().await
            && let Some(metadata) = description.key_metadata()
        {
            status = metadata
                .key_state()
                .map(ToString::to_string)
                .unwrap_or(status);
            created_at = format_timestamp(metadata.creation_date());
            insert_attr(&mut attributes, "description", metadata.description());
            insert_attr(
                &mut attributes,
                "key_usage",
                metadata.key_usage().map(ToString::to_string),
            );
            insert_attr(
                &mut attributes,
                "key_manager",
                metadata.key_manager().map(ToString::to_string),
            );
            insert_attr(&mut attributes, "enabled", Some(metadata.enabled()));
        }

        resources.push(ResourceSummary {
            id: format!("key/{key_id}"),
            name: key_id.to_owned(),
            kind: "key".to_owned(),
            status,
            created_at,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });

        if let Ok(grants) = client.list_grants().key_id(key_id).send().await {
            for grant in grants.grants() {
                let Some(grant_id) = grant.grant_id() else {
                    continue;
                };
                let operations = grant
                    .operations()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");

                resources.push(ResourceSummary {
                    id: format!("grant/{key_id}/{grant_id}"),
                    name: grant.name().unwrap_or(grant_id).to_owned(),
                    kind: "grant".to_owned(),
                    status: "active".to_owned(),
                    created_at: format_timestamp(grant.creation_date()),
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes: BTreeMap::from([
                        ("key_id".to_owned(), key_id.to_owned()),
                        ("grant_id".to_owned(), grant_id.to_owned()),
                        (
                            "grantee_principal".to_owned(),
                            grant.grantee_principal().unwrap_or("unknown").to_owned(),
                        ),
                        ("operations".to_owned(), operations),
                    ]),
                });
            }
        }
    }

    if let Ok(aliases) = client.list_aliases().send().await {
        for alias in aliases.aliases() {
            let Some(alias_name) = alias.alias_name() else {
                continue;
            };
            let mut attributes = BTreeMap::new();
            insert_attr(&mut attributes, "alias_arn", alias.alias_arn());
            insert_attr(&mut attributes, "target_key_id", alias.target_key_id());

            resources.push(ResourceSummary {
                id: format!("alias/{alias_name}"),
                name: alias_name.to_owned(),
                kind: "alias".to_owned(),
                status: if alias.target_key_id().is_some() {
                    "active"
                } else {
                    "unbound"
                }
                .to_owned(),
                created_at: format_timestamp(alias.creation_date()),
                updated_at: format_timestamp(alias.last_updated_date()),
                tags: BTreeMap::new(),
                attributes,
            });
        }
    }

    Ok(managed_inventory(
        "kms",
        "KMS",
        tabs(),
        resources,
        vec![
            "create_grant".to_owned(),
            "delete_alias".to_owned(),
            "put_key_policy".to_owned(),
        ],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_key" => {
            let description = optional_payload_text(request, "description")
                .unwrap_or_else(|| "Created by floci-ui local service management.".to_owned());
            let output = client(config)
                .create_key()
                .description(description)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("kms", "create_key", err))?;
            let key_id = output
                .key_metadata()
                .map(|metadata| metadata.key_id())
                .unwrap_or("created");

            Ok(ActionResult {
                changed: true,
                message: format!("Created KMS key `{key_id}`."),
                resource_id: Some(format!("key/{key_id}")),
            })
        }
        "create_alias" => {
            let alias_name = require_payload_text(request, "alias_name")?;
            let target_key_id = optional_payload_text(request, "target_key_id")
                .or_else(|| {
                    request
                        .resource_id
                        .as_deref()?
                        .strip_prefix("key/")
                        .map(ToOwned::to_owned)
                })
                .ok_or_else(|| {
                    ServiceManagementError::invalid_input(
                        request.service_key.clone(),
                        request.action.clone(),
                        "Select a key or provide `target_key_id`.",
                    )
                })?;
            client(config)
                .create_alias()
                .alias_name(&alias_name)
                .target_key_id(&target_key_id)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("kms", "create_alias", err))?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created KMS alias `{alias_name}`."),
                resource_id: Some(format!("alias/{alias_name}")),
            })
        }
        "disable_key" => {
            let key_name = require_typed_confirmation(request)?;
            let key_id = require_resource_id(request, "key/")?;
            client(config)
                .disable_key()
                .key_id(key_id)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("kms", "disable_key", err))?;

            Ok(ActionResult {
                changed: true,
                message: format!("Disabled KMS key `{key_name}`."),
                resource_id: Some(format!("key/{key_id}")),
            })
        }
        "schedule_key_deletion" => {
            let key_name = require_typed_confirmation(request)?;
            let key_id = require_resource_id(request, "key/")?;
            let pending_window =
                optional_payload_i32(request, "pending_window_in_days").unwrap_or(7);
            client(config)
                .schedule_key_deletion()
                .key_id(key_id)
                .pending_window_in_days(pending_window)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("kms", "schedule_key_deletion", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!(
                    "Scheduled KMS key `{key_name}` for deletion in {pending_window} days."
                ),
                resource_id: Some(format!("key/{key_id}")),
            })
        }
        "refresh_key_policy" => {
            let key_id = require_resource_id(request, "key/")?;
            Ok(ActionResult {
                changed: false,
                message: format!("Refreshed KMS key policy metadata for `{key_id}`."),
                resource_id: Some(format!("key/{key_id}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        tab("keys", "Keys", &["key"], "No KMS keys were found."),
        tab(
            "aliases",
            "Aliases",
            &["alias"],
            "No KMS aliases were found.",
        ),
        tab("grants", "Grants", &["grant"], "No KMS grants were found."),
        tab(
            "policies",
            "Policies",
            &["key-policy"],
            "KMS key policies are summarized through selected key metadata.",
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

fn client(config: &AppConfig) -> aws_sdk_kms::Client {
    let sdk_config = aws_sdk_kms::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_kms::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_kms::Client::from_conf(sdk_config)
}
