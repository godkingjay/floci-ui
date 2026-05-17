use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        actions::require_typed_confirmation,
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
    let secrets = client.list_secrets().send().await.map_err(|err| {
        ServiceManagementError::client_error("secretsmanager", "list_secrets", err)
    })?;
    let mut resources = Vec::new();

    for secret in secrets.secret_list() {
        let Some(name) = secret.name() else {
            continue;
        };
        let arn = secret.arn().unwrap_or(name);
        let mut attributes = BTreeMap::new();
        attributes.insert("arn".to_owned(), arn.to_owned());
        attributes.insert("secret_value".to_owned(), redacted_value_marker());
        insert_attr(&mut attributes, "description", secret.description());
        insert_attr(&mut attributes, "kms_key_id", secret.kms_key_id());
        insert_attr(
            &mut attributes,
            "rotation_enabled",
            secret.rotation_enabled(),
        );
        insert_attr(
            &mut attributes,
            "last_accessed_date",
            format_timestamp(secret.last_accessed_date()),
        );
        insert_attr(
            &mut attributes,
            "last_rotated_date",
            format_timestamp(secret.last_rotated_date()),
        );

        resources.push(ResourceSummary {
            id: format!("secret/{arn}"),
            name: name.to_owned(),
            kind: "secret".to_owned(),
            status: if secret.deleted_date().is_some() {
                "deleting"
            } else if secret.rotation_enabled() == Some(true) {
                "rotation-enabled"
            } else {
                "active"
            }
            .to_owned(),
            created_at: format_timestamp(secret.created_date()),
            updated_at: format_timestamp(secret.last_changed_date()),
            tags: BTreeMap::new(),
            attributes,
        });

        if let Ok(versions) = client
            .list_secret_version_ids()
            .secret_id(arn)
            .include_deprecated(false)
            .send()
            .await
        {
            for version in versions.versions() {
                let Some(version_id) = version.version_id() else {
                    continue;
                };
                let stages = version
                    .version_stages()
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(", ");
                resources.push(ResourceSummary {
                    id: format!("secret-version/{arn}/{version_id}"),
                    name: version_id.to_owned(),
                    kind: "secret-version".to_owned(),
                    status: if stages.contains("AWSCURRENT") {
                        "current"
                    } else {
                        "versioned"
                    }
                    .to_owned(),
                    created_at: format_timestamp(version.created_date()),
                    updated_at: format_timestamp(version.last_accessed_date()),
                    tags: BTreeMap::new(),
                    attributes: BTreeMap::from([
                        ("secret_id".to_owned(), arn.to_owned()),
                        ("version_id".to_owned(), version_id.to_owned()),
                        ("version_stages".to_owned(), stages),
                        ("secret_value".to_owned(), redacted_value_marker()),
                    ]),
                });
            }
        }

        resources.push(ResourceSummary {
            id: format!("rotation/{arn}"),
            name: format!("{name} rotation"),
            kind: "rotation".to_owned(),
            status: if secret.rotation_enabled() == Some(true) {
                "enabled"
            } else {
                "disabled"
            }
            .to_owned(),
            created_at: format_timestamp(secret.created_date()),
            updated_at: format_timestamp(secret.last_rotated_date()),
            tags: BTreeMap::new(),
            attributes: BTreeMap::from([
                ("secret_id".to_owned(), arn.to_owned()),
                (
                    "rotation_lambda_arn".to_owned(),
                    secret
                        .rotation_lambda_arn()
                        .unwrap_or("not configured")
                        .to_owned(),
                ),
            ]),
        });
    }

    Ok(managed_inventory(
        "secretsmanager",
        "Secrets Manager",
        tabs(),
        resources,
        vec![
            "create_secret".to_owned(),
            "put_secret_value".to_owned(),
            "delete_secret".to_owned(),
        ],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "reveal_secret_value" => {
            let secret_id = require_resource_id(request, "secret/")?;
            let output = client(config)
                .get_secret_value()
                .secret_id(secret_id)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("secretsmanager", "get_secret_value", err)
                })?;
            let value_kind = if output.secret_string().is_some() {
                "string"
            } else if output.secret_binary().is_some() {
                "binary"
            } else {
                "empty"
            };
            let value_length = output
                .secret_string()
                .map(str::len)
                .or_else(|| output.secret_binary().map(|blob| blob.as_ref().len()))
                .unwrap_or(0);

            Ok(ActionResult {
                changed: false,
                message: format!(
                    "Secret value was fetched for `{}` and immediately cleared from the UI action state. Type: {value_kind}; length: {value_length} bytes. The value is not stored in inventory, logs, or screenshots.",
                    output.name().unwrap_or(secret_id),
                ),
                resource_id: Some(format!("secret/{secret_id}")),
            })
        }
        "rotate_secret" => {
            let secret_name = require_typed_confirmation(request)?;
            let secret_id = require_resource_id(request, "secret/")?;
            client(config)
                .rotate_secret()
                .secret_id(secret_id)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("secretsmanager", "rotate_secret", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Started rotation for Secrets Manager secret `{secret_name}`."),
                resource_id: Some(format!("secret/{secret_id}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        tab("secrets", "Secrets", &["secret"], "No secrets were found."),
        tab(
            "versions",
            "Versions",
            &["secret-version"],
            "No secret versions were found.",
        ),
        tab(
            "rotation",
            "Rotation",
            &["rotation"],
            "No secret rotation metadata is available.",
        ),
        tab(
            "tags",
            "Tags",
            &["tag-set"],
            "Secret tags are folded into resource metadata.",
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

fn client(config: &AppConfig) -> aws_sdk_secretsmanager::Client {
    let sdk_config = aws_sdk_secretsmanager::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_secretsmanager::config::Region::new(
            config.region.clone(),
        ))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_secretsmanager::Client::from_conf(sdk_config)
}
