use std::collections::BTreeMap;

use aws_sdk_transfer::types::{IdentityProviderType, Protocol};

use crate::{
    config::AppConfig,
    service_management::{
        actions::{require_name_payload, require_typed_confirmation},
        adapters::network_observability::{
            format_timestamp, insert_attr, local_credentials, managed_inventory,
            optional_payload_text, redacted_value_marker, require_payload_text,
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
    let servers = client
        .list_servers()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("transfer", "list_servers", err))?;
    let mut resources = Vec::new();

    for server in servers.servers() {
        let Some(server_id) = server.server_id() else {
            continue;
        };
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "arn", Some(server.arn()));
        if let Some(domain) = server.domain() {
            insert_attr(&mut attributes, "domain", Some(domain.as_str()));
        }
        if let Some(endpoint_type) = server.endpoint_type() {
            insert_attr(
                &mut attributes,
                "endpoint_type",
                Some(endpoint_type.as_str()),
            );
        }
        if let Some(identity_provider_type) = server.identity_provider_type() {
            insert_attr(
                &mut attributes,
                "identity_provider_type",
                Some(identity_provider_type.as_str()),
            );
        }
        if server.logging_role().is_some() {
            insert_attr(
                &mut attributes,
                "logging_role",
                Some(redacted_value_marker()),
            );
        }

        resources.push(ResourceSummary {
            id: format!("server/{server_id}"),
            name: server_id.to_owned(),
            kind: "server".to_owned(),
            status: server
                .state()
                .map(|state| state.as_str().to_owned())
                .unwrap_or_else(|| "available".to_owned()),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });

        load_users(&client, server_id, &mut resources).await;
        load_host_keys(&client, server_id, &mut resources).await;
    }

    load_workflows(&client, &mut resources).await;

    Ok(managed_inventory(
        "transfer",
        "Transfer Family",
        tabs(),
        resources,
        vec!["identity_details_redacted".to_owned()],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_server" => {
            let protocol = Protocol::from(
                optional_payload_text(request, "protocol")
                    .unwrap_or_else(|| "SFTP".to_owned())
                    .to_ascii_uppercase()
                    .as_str(),
            );
            let output = client(config)
                .create_server()
                .identity_provider_type(IdentityProviderType::ServiceManaged)
                .protocols(protocol)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("transfer", "create_server", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created Transfer server `{}`.", output.server_id()),
                resource_id: Some(format!("server/{}", output.server_id())),
            })
        }
        "create_user" => {
            let server_id = require_resource_id(request, "server/")?;
            let user_name = require_name_payload(request, "user_name")?;
            let role_arn = require_payload_text(request, "role_arn")?;
            let mut builder = client(config)
                .create_user()
                .server_id(server_id)
                .user_name(&user_name)
                .role(&role_arn);
            if let Some(home_directory) = optional_payload_text(request, "home_directory") {
                builder = builder.home_directory(home_directory);
            }
            let output = builder.send().await.map_err(|err| {
                ServiceManagementError::client_error("transfer", "create_user", err)
            })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created Transfer user `{}`.", output.user_name()),
                resource_id: Some(format!(
                    "user/{}/{}",
                    output.server_id(),
                    output.user_name()
                )),
            })
        }
        "delete_user" => {
            let user_name = require_typed_confirmation(request)?;
            let (server_id, selected_user) = selected_user(request)?;

            client(config)
                .delete_user()
                .server_id(server_id)
                .user_name(selected_user)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("transfer", "delete_user", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted Transfer user `{user_name}`."),
                resource_id: Some(format!("user/{server_id}/{selected_user}")),
            })
        }
        "delete_server" => {
            let server_name = require_typed_confirmation(request)?;
            let server_id = require_resource_id(request, "server/")?;

            client(config)
                .delete_server()
                .server_id(server_id)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("transfer", "delete_server", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted Transfer server `{server_name}`."),
                resource_id: Some(format!("server/{server_id}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "servers".to_owned(),
            label: "Servers".to_owned(),
            kinds: vec!["server".to_owned()],
            empty_message: "No Transfer Family servers were found in the local emulator."
                .to_owned(),
        },
        ResourceTab {
            key: "users".to_owned(),
            label: "Users".to_owned(),
            kinds: vec!["user".to_owned()],
            empty_message: "No Transfer Family users are loaded.".to_owned(),
        },
        ResourceTab {
            key: "workflows".to_owned(),
            label: "Workflows".to_owned(),
            kinds: vec!["workflow".to_owned()],
            empty_message: "No Transfer Family workflows are loaded.".to_owned(),
        },
        ResourceTab {
            key: "host-keys".to_owned(),
            label: "Host Keys".to_owned(),
            kinds: vec!["host-key".to_owned()],
            empty_message: "No Transfer Family host keys are loaded.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_transfer::Client {
    let sdk_config = aws_sdk_transfer::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_transfer::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_transfer::Client::from_conf(sdk_config)
}

async fn load_users(
    client: &aws_sdk_transfer::Client,
    server_id: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client.list_users().server_id(server_id).send().await else {
        return;
    };

    for user in output.users() {
        let Some(user_name) = user.user_name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "server_id", Some(server_id));
        insert_attr(&mut attributes, "arn", Some(user.arn()));
        if user.role().is_some() {
            insert_attr(&mut attributes, "role", Some(redacted_value_marker()));
        }
        if user.home_directory().is_some() {
            insert_attr(
                &mut attributes,
                "home_directory",
                Some(redacted_value_marker()),
            );
        }
        if let Some(home_directory_type) = user.home_directory_type() {
            insert_attr(
                &mut attributes,
                "home_directory_type",
                Some(home_directory_type.as_str()),
            );
        }

        resources.push(ResourceSummary {
            id: format!("user/{server_id}/{user_name}"),
            name: user_name.to_owned(),
            kind: "user".to_owned(),
            status: "configured".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

async fn load_workflows(client: &aws_sdk_transfer::Client, resources: &mut Vec<ResourceSummary>) {
    let Ok(output) = client.list_workflows().send().await else {
        return;
    };

    for workflow in output.workflows() {
        let Some(workflow_id) = workflow.workflow_id() else {
            continue;
        };
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "description", workflow.description());
        insert_attr(&mut attributes, "arn", workflow.arn());

        resources.push(ResourceSummary {
            id: format!("workflow/{workflow_id}"),
            name: workflow_id.to_owned(),
            kind: "workflow".to_owned(),
            status: "configured".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

async fn load_host_keys(
    client: &aws_sdk_transfer::Client,
    server_id: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client.list_host_keys().server_id(server_id).send().await else {
        return;
    };

    for host_key in output.host_keys() {
        let Some(host_key_id) = host_key.host_key_id() else {
            continue;
        };
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "server_id", Some(server_id));
        insert_attr(&mut attributes, "arn", Some(host_key.arn()));
        insert_attr(&mut attributes, "description", host_key.description());
        insert_attr(&mut attributes, "public_key", Some(redacted_value_marker()));

        resources.push(ResourceSummary {
            id: format!("host-key/{server_id}/{host_key_id}"),
            name: host_key_id.to_owned(),
            kind: "host-key".to_owned(),
            status: "imported".to_owned(),
            created_at: format_timestamp(host_key.date_imported()),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

fn selected_user(request: &ServiceActionRequest) -> Result<(&str, &str), ServiceManagementError> {
    let selected = require_resource_id(request, "user/")?;
    let mut parts = selected.splitn(2, '/');
    let server_id = parts.next().unwrap_or_default();
    let user_name = parts.next().unwrap_or_default();

    if server_id.is_empty() || user_name.is_empty() {
        return Err(ServiceManagementError::invalid_input(
            request.service_key.clone(),
            request.action.clone(),
            "Select a Transfer user before running this action.",
        ));
    }

    Ok((server_id, user_name))
}
