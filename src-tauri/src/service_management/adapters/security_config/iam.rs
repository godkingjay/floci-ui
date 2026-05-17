use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        actions::{require_name_payload, require_typed_confirmation},
        adapters::security_config::{
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

const DEFAULT_ASSUME_ROLE_POLICY: &str = r#"{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": { "Service": "lambda.amazonaws.com" },
      "Action": "sts:AssumeRole"
    }
  ]
}"#;

const DEFAULT_POLICY_DOCUMENT: &str = r#"{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": "*",
      "Resource": "*"
    }
  ]
}"#;

#[allow(clippy::too_many_lines)]
pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let client = client(config);
    let mut resources = Vec::new();

    let users = client
        .list_users()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("iam", "list_users", err))?;
    for user in users.users() {
        let user_name = user.user_name();
        let mut attributes = BTreeMap::new();
        attributes.insert("arn".to_owned(), user.arn().to_owned());
        attributes.insert("user_id".to_owned(), user.user_id().to_owned());
        attributes.insert("path".to_owned(), user.path().to_owned());
        insert_attr(
            &mut attributes,
            "password_last_used",
            format_timestamp(user.password_last_used()),
        );

        resources.push(ResourceSummary {
            id: format!("user/{user_name}"),
            name: user_name.to_owned(),
            kind: "user".to_owned(),
            status: "active".to_owned(),
            created_at: format_timestamp(Some(user.create_date())),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });

        if let Ok(keys) = client.list_access_keys().user_name(user_name).send().await {
            for access_key in keys.access_key_metadata() {
                let Some(access_key_id) = access_key.access_key_id() else {
                    continue;
                };
                let owner = access_key.user_name().unwrap_or(user_name);
                resources.push(ResourceSummary {
                    id: format!("access-key/{owner}/{access_key_id}"),
                    name: access_key_id.to_owned(),
                    kind: "access-key".to_owned(),
                    status: access_key
                        .status()
                        .map_or_else(|| "unknown".to_owned(), ToString::to_string),
                    created_at: format_timestamp(access_key.create_date()),
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes: BTreeMap::from([
                        ("user_name".to_owned(), owner.to_owned()),
                        ("access_key_id".to_owned(), access_key_id.to_owned()),
                        ("secret_access_key".to_owned(), "redacted".to_owned()),
                    ]),
                });
            }
        }
    }

    let roles = client
        .list_roles()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("iam", "list_roles", err))?;
    for role in roles.roles() {
        let role_name = role.role_name();
        let mut attributes = BTreeMap::new();
        attributes.insert("arn".to_owned(), role.arn().to_owned());
        attributes.insert("role_id".to_owned(), role.role_id().to_owned());
        attributes.insert("path".to_owned(), role.path().to_owned());
        insert_attr(&mut attributes, "description", role.description());
        insert_attr(
            &mut attributes,
            "max_session_duration",
            role.max_session_duration(),
        );
        if role.assume_role_policy_document().is_some() {
            attributes.insert(
                "assume_role_policy_document".to_owned(),
                "available in AWS metadata; not expanded in inventory".to_owned(),
            );
        }

        resources.push(ResourceSummary {
            id: format!("role/{role_name}"),
            name: role_name.to_owned(),
            kind: "role".to_owned(),
            status: "available".to_owned(),
            created_at: format_timestamp(Some(role.create_date())),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    let groups = client
        .list_groups()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("iam", "list_groups", err))?;
    for group in groups.groups() {
        resources.push(ResourceSummary {
            id: format!("group/{}", group.group_name()),
            name: group.group_name().to_owned(),
            kind: "group".to_owned(),
            status: "available".to_owned(),
            created_at: format_timestamp(Some(group.create_date())),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes: BTreeMap::from([
                ("arn".to_owned(), group.arn().to_owned()),
                ("group_id".to_owned(), group.group_id().to_owned()),
                ("path".to_owned(), group.path().to_owned()),
            ]),
        });
    }

    let policies = client
        .list_policies()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("iam", "list_policies", err))?;
    for policy in policies.policies() {
        let Some(arn) = policy.arn() else {
            continue;
        };
        let name = policy
            .policy_name()
            .unwrap_or_else(|| resource_name_from_arn(arn));
        let mut attributes = BTreeMap::new();
        attributes.insert("arn".to_owned(), arn.to_owned());
        insert_attr(&mut attributes, "policy_id", policy.policy_id());
        insert_attr(
            &mut attributes,
            "default_version_id",
            policy.default_version_id(),
        );
        insert_attr(
            &mut attributes,
            "attachment_count",
            policy.attachment_count(),
        );
        attributes.insert(
            "is_attachable".to_owned(),
            policy.is_attachable().to_string(),
        );
        insert_attr(&mut attributes, "description", policy.description());

        resources.push(ResourceSummary {
            id: format!("policy/{arn}"),
            name: name.to_owned(),
            kind: "policy".to_owned(),
            status: "available".to_owned(),
            created_at: format_timestamp(policy.create_date()),
            updated_at: format_timestamp(policy.update_date()),
            tags: BTreeMap::new(),
            attributes,
        });
    }

    if let Ok(instance_profiles) = client.list_instance_profiles().send().await {
        for profile in instance_profiles.instance_profiles() {
            let role_names = profile
                .roles()
                .iter()
                .map(|role| role.role_name())
                .collect::<Vec<_>>()
                .join(", ");

            resources.push(ResourceSummary {
                id: format!("instance-profile/{}", profile.instance_profile_name()),
                name: profile.instance_profile_name().to_owned(),
                kind: "instance-profile".to_owned(),
                status: "available".to_owned(),
                created_at: format_timestamp(Some(profile.create_date())),
                updated_at: None,
                tags: BTreeMap::new(),
                attributes: BTreeMap::from([
                    ("arn".to_owned(), profile.arn().to_owned()),
                    (
                        "instance_profile_id".to_owned(),
                        profile.instance_profile_id().to_owned(),
                    ),
                    ("path".to_owned(), profile.path().to_owned()),
                    ("roles".to_owned(), role_names),
                ]),
            });
        }
    }

    Ok(managed_inventory(
        "iam",
        "IAM",
        tabs(),
        resources,
        vec![
            "attach_policy".to_owned(),
            "detach_policy".to_owned(),
            "delete_group".to_owned(),
        ],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_user" => {
            let user_name = require_name_payload(request, "user_name")?;
            client(config)
                .create_user()
                .user_name(&user_name)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("iam", "create_user", err))?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created IAM user `{user_name}`."),
                resource_id: Some(format!("user/{user_name}")),
            })
        }
        "delete_user" => {
            let user_name = require_typed_confirmation(request)?;
            client(config)
                .delete_user()
                .user_name(&user_name)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("iam", "delete_user", err))?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted IAM user `{user_name}`."),
                resource_id: Some(format!("user/{user_name}")),
            })
        }
        "create_role" => {
            let role_name = require_name_payload(request, "role_name")?;
            let assume_role_policy_document = require_json_payload_text(
                request,
                "assume_role_policy_document",
                DEFAULT_ASSUME_ROLE_POLICY,
            )?;
            client(config)
                .create_role()
                .role_name(&role_name)
                .assume_role_policy_document(assume_role_policy_document)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("iam", "create_role", err))?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created IAM role `{role_name}`."),
                resource_id: Some(format!("role/{role_name}")),
            })
        }
        "delete_role" => {
            let role_name = require_typed_confirmation(request)?;
            client(config)
                .delete_role()
                .role_name(&role_name)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("iam", "delete_role", err))?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted IAM role `{role_name}`."),
                resource_id: Some(format!("role/{role_name}")),
            })
        }
        "create_policy" => {
            let policy_name = require_name_payload(request, "policy_name")?;
            let policy_document =
                require_json_payload_text(request, "policy_document", DEFAULT_POLICY_DOCUMENT)?;
            let output = client(config)
                .create_policy()
                .policy_name(&policy_name)
                .policy_document(policy_document)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("iam", "create_policy", err))?;
            let arn = output
                .policy()
                .and_then(|policy| policy.arn())
                .map_or_else(|| format!("local/{policy_name}"), ToOwned::to_owned);

            Ok(ActionResult {
                changed: true,
                message: format!("Created IAM policy `{policy_name}`."),
                resource_id: Some(format!("policy/{arn}")),
            })
        }
        "delete_policy" => {
            let policy_name = require_typed_confirmation(request)?;
            let policy_arn = require_resource_id(request, "policy/")?;
            client(config)
                .delete_policy()
                .policy_arn(policy_arn)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("iam", "delete_policy", err))?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted IAM policy `{policy_name}`."),
                resource_id: Some(format!("policy/{policy_arn}")),
            })
        }
        "create_access_key" => {
            let user_name = optional_payload_text(request, "user_name")
                .or_else(|| {
                    request
                        .resource_id
                        .as_deref()?
                        .strip_prefix("user/")
                        .map(ToOwned::to_owned)
                })
                .ok_or_else(|| {
                    ServiceManagementError::invalid_input(
                        request.service_key.clone(),
                        request.action.clone(),
                        "Select a user or provide `user_name`.",
                    )
                })?;
            let output = client(config)
                .create_access_key()
                .user_name(&user_name)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("iam", "create_access_key", err)
                })?;
            let access_key_id = output
                .access_key()
                .map(|key| key.access_key_id())
                .unwrap_or("created");

            Ok(ActionResult {
                changed: true,
                message: format!(
                    "Created IAM access key `{access_key_id}` for `{user_name}`. Secret access key was received from the local emulator and immediately discarded."
                ),
                resource_id: Some(format!("access-key/{user_name}/{access_key_id}")),
            })
        }
        "delete_access_key" => {
            let access_key_name = require_typed_confirmation(request)?;
            let key_path = require_resource_id(request, "access-key/")?;
            let mut parts = key_path.splitn(2, '/');
            let user_name = parts.next().unwrap_or_default();
            let access_key_id = parts.next().unwrap_or(key_path);
            client(config)
                .delete_access_key()
                .user_name(user_name)
                .access_key_id(access_key_id)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("iam", "delete_access_key", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted IAM access key `{access_key_name}`."),
                resource_id: Some(format!("access-key/{user_name}/{access_key_id}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        tab("users", "Users", &["user"], "No IAM users were found."),
        tab("roles", "Roles", &["role"], "No IAM roles were found."),
        tab("groups", "Groups", &["group"], "No IAM groups were found."),
        tab(
            "policies",
            "Policies",
            &["policy"],
            "No IAM policies were found.",
        ),
        tab(
            "access-keys",
            "Access Keys",
            &["access-key"],
            "No IAM access keys were found.",
        ),
        tab(
            "instance-profiles",
            "Instance Profiles",
            &["instance-profile"],
            "No IAM instance profiles were found.",
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

fn client(config: &AppConfig) -> aws_sdk_iam::Client {
    let sdk_config = aws_sdk_iam::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_iam::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_iam::Client::from_conf(sdk_config)
}
