use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        actions::{require_name_payload, require_typed_confirmation},
        adapters::security_config::{
            format_timestamp, insert_attr, local_credentials, managed_inventory,
            optional_payload_text, require_resource_id, unsupported_action,
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
    let pools = client
        .list_user_pools()
        .max_results(60)
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("cognito", "list_user_pools", err))?;
    let mut resources = Vec::new();

    for pool in pools.user_pools() {
        let Some(pool_id) = pool.id() else {
            continue;
        };
        let pool_name = pool.name().unwrap_or(pool_id);
        let mut attributes = BTreeMap::new();
        attributes.insert("user_pool_id".to_owned(), pool_id.to_owned());
        insert_attr(
            &mut attributes,
            "lambda_config",
            pool.lambda_config().map(|_| "present"),
        );

        resources.push(ResourceSummary {
            id: format!("user-pool/{pool_id}"),
            name: pool_name.to_owned(),
            kind: "user-pool".to_owned(),
            status: "available".to_owned(),
            created_at: format_timestamp(pool.creation_date()),
            updated_at: format_timestamp(pool.last_modified_date()),
            tags: BTreeMap::new(),
            attributes,
        });

        if let Ok(clients) = client
            .list_user_pool_clients()
            .user_pool_id(pool_id)
            .max_results(60)
            .send()
            .await
        {
            for app_client in clients.user_pool_clients() {
                let Some(client_id) = app_client.client_id() else {
                    continue;
                };
                resources.push(ResourceSummary {
                    id: format!("app-client/{pool_id}/{client_id}"),
                    name: app_client.client_name().unwrap_or(client_id).to_owned(),
                    kind: "app-client".to_owned(),
                    status: "available".to_owned(),
                    created_at: None,
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes: BTreeMap::from([
                        ("user_pool_id".to_owned(), pool_id.to_owned()),
                        ("client_id".to_owned(), client_id.to_owned()),
                        ("client_secret".to_owned(), "redacted".to_owned()),
                    ]),
                });
            }
        }

        if let Ok(users) = client.list_users().user_pool_id(pool_id).send().await {
            for user in users.users() {
                let Some(username) = user.username() else {
                    continue;
                };
                let mut attributes =
                    BTreeMap::from([("user_pool_id".to_owned(), pool_id.to_owned())]);
                insert_attr(&mut attributes, "enabled", Some(user.enabled()));

                resources.push(ResourceSummary {
                    id: format!("user/{pool_id}/{username}"),
                    name: username.to_owned(),
                    kind: "user".to_owned(),
                    status: user
                        .user_status()
                        .map_or_else(|| "unknown".to_owned(), ToString::to_string),
                    created_at: format_timestamp(user.user_create_date()),
                    updated_at: format_timestamp(user.user_last_modified_date()),
                    tags: BTreeMap::new(),
                    attributes,
                });
            }
        }

        if let Ok(groups) = client.list_groups().user_pool_id(pool_id).send().await {
            for group in groups.groups() {
                let Some(group_name) = group.group_name() else {
                    continue;
                };
                let mut attributes =
                    BTreeMap::from([("user_pool_id".to_owned(), pool_id.to_owned())]);
                insert_attr(&mut attributes, "description", group.description());
                insert_attr(&mut attributes, "role_arn", group.role_arn());

                resources.push(ResourceSummary {
                    id: format!("group/{pool_id}/{group_name}"),
                    name: group_name.to_owned(),
                    kind: "group".to_owned(),
                    status: "available".to_owned(),
                    created_at: format_timestamp(group.creation_date()),
                    updated_at: format_timestamp(group.last_modified_date()),
                    tags: BTreeMap::new(),
                    attributes,
                });
            }
        }

        if let Ok(providers) = client
            .list_identity_providers()
            .user_pool_id(pool_id)
            .send()
            .await
        {
            for provider in providers.providers() {
                let Some(provider_name) = provider.provider_name() else {
                    continue;
                };
                let mut attributes =
                    BTreeMap::from([("user_pool_id".to_owned(), pool_id.to_owned())]);
                insert_attr(
                    &mut attributes,
                    "provider_type",
                    provider.provider_type().map(ToString::to_string),
                );

                resources.push(ResourceSummary {
                    id: format!("identity-provider/{pool_id}/{provider_name}"),
                    name: provider_name.to_owned(),
                    kind: "identity-provider".to_owned(),
                    status: "available".to_owned(),
                    created_at: format_timestamp(provider.creation_date()),
                    updated_at: format_timestamp(provider.last_modified_date()),
                    tags: BTreeMap::new(),
                    attributes,
                });
            }
        }
    }

    Ok(managed_inventory(
        "cognito",
        "Cognito",
        tabs(),
        resources,
        vec!["create_group".to_owned(), "delete_group".to_owned()],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_user_pool" => {
            let pool_name = require_name_payload(request, "pool_name")?;
            let output = client(config)
                .create_user_pool()
                .pool_name(&pool_name)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("cognito", "create_user_pool", err)
                })?;
            let pool_id = output
                .user_pool()
                .and_then(|pool| pool.id())
                .map_or_else(|| pool_name.clone(), ToOwned::to_owned);

            Ok(ActionResult {
                changed: true,
                message: format!("Created Cognito user pool `{pool_name}`."),
                resource_id: Some(format!("user-pool/{pool_id}")),
            })
        }
        "delete_user_pool" => {
            let pool_name = require_typed_confirmation(request)?;
            let pool_id = require_resource_id(request, "user-pool/")?;
            client(config)
                .delete_user_pool()
                .user_pool_id(pool_id)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("cognito", "delete_user_pool", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted Cognito user pool `{pool_name}`."),
                resource_id: Some(format!("user-pool/{pool_id}")),
            })
        }
        "create_user_pool_client" => {
            let client_name = optional_payload_text(request, "client_name")
                .unwrap_or_else(|| "floci-local-client".to_owned());
            let pool_id = require_resource_id(request, "user-pool/")?;
            let output = client(config)
                .create_user_pool_client()
                .user_pool_id(pool_id)
                .client_name(&client_name)
                .generate_secret(false)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("cognito", "create_user_pool_client", err)
                })?;
            let client_id = output
                .user_pool_client()
                .and_then(|app_client| app_client.client_id())
                .map_or_else(|| client_name.clone(), ToOwned::to_owned);

            Ok(ActionResult {
                changed: true,
                message: format!("Created Cognito app client `{client_name}`."),
                resource_id: Some(format!("app-client/{pool_id}/{client_id}")),
            })
        }
        "delete_user_pool_client" => {
            let client_name = require_typed_confirmation(request)?;
            let key_path = require_resource_id(request, "app-client/")?;
            let mut parts = key_path.splitn(2, '/');
            let pool_id = parts.next().unwrap_or_default();
            let client_id = parts.next().unwrap_or(key_path);
            client(config)
                .delete_user_pool_client()
                .user_pool_id(pool_id)
                .client_id(client_id)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("cognito", "delete_user_pool_client", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted Cognito app client `{client_name}`."),
                resource_id: Some(format!("app-client/{pool_id}/{client_id}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        tab(
            "user-pools",
            "User Pools",
            &["user-pool"],
            "No Cognito user pools were found.",
        ),
        tab(
            "app-clients",
            "App Clients",
            &["app-client"],
            "No Cognito app clients were found.",
        ),
        tab("users", "Users", &["user"], "No Cognito users were found."),
        tab(
            "groups",
            "Groups",
            &["group"],
            "No Cognito groups were found.",
        ),
        tab(
            "providers",
            "Providers",
            &["identity-provider"],
            "No Cognito identity providers were found.",
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

fn client(config: &AppConfig) -> aws_sdk_cognitoidentityprovider::Client {
    let sdk_config = aws_sdk_cognitoidentityprovider::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_cognitoidentityprovider::config::Region::new(
            config.region.clone(),
        ))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_cognitoidentityprovider::Client::from_conf(sdk_config)
}
