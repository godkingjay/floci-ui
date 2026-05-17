use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        adapters::compute_build::{
            insert_attr, local_credentials, managed_inventory, optional_payload_i32,
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
    let output = client
        .describe_auto_scaling_groups()
        .send()
        .await
        .map_err(|err| {
            ServiceManagementError::client_error("autoscaling", "describe_auto_scaling_groups", err)
        })?;
    let mut resources = Vec::new();

    for group in output.auto_scaling_groups() {
        let Some(group_name) = group.auto_scaling_group_name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "min_size", group.min_size());
        insert_attr(&mut attributes, "max_size", group.max_size());
        insert_attr(
            &mut attributes,
            "desired_capacity",
            group.desired_capacity(),
        );
        insert_attr(
            &mut attributes,
            "launch_configuration",
            group.launch_configuration_name(),
        );
        insert_attr(
            &mut attributes,
            "launch_template",
            group
                .launch_template()
                .and_then(|template| template.launch_template_name()),
        );

        resources.push(ResourceSummary {
            id: format!("auto-scaling-group/{group_name}"),
            name: group_name.to_owned(),
            kind: "auto-scaling-group".to_owned(),
            status: "available".to_owned(),
            created_at: super::format_timestamp(group.created_time()),
            updated_at: None,
            tags: tags_to_map(group.tags()),
            attributes,
        });

        for instance in group.instances() {
            let Some(instance_id) = instance.instance_id() else {
                continue;
            };
            let mut attributes =
                BTreeMap::from([("auto_scaling_group".to_owned(), group_name.to_owned())]);
            insert_attr(
                &mut attributes,
                "lifecycle_state",
                instance.lifecycle_state(),
            );
            insert_attr(&mut attributes, "health_status", instance.health_status());
            insert_attr(
                &mut attributes,
                "availability_zone",
                instance.availability_zone(),
            );

            resources.push(ResourceSummary {
                id: format!("scaling-instance/{group_name}/{instance_id}"),
                name: instance_id.to_owned(),
                kind: "scaling-instance".to_owned(),
                status: instance
                    .health_status()
                    .unwrap_or("unknown")
                    .to_ascii_lowercase(),
                created_at: None,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes,
            });
        }

        if let Ok(hooks) = client
            .describe_lifecycle_hooks()
            .auto_scaling_group_name(group_name)
            .send()
            .await
        {
            for hook in hooks.lifecycle_hooks() {
                let Some(hook_name) = hook.lifecycle_hook_name() else {
                    continue;
                };
                resources.push(ResourceSummary {
                    id: format!("lifecycle-hook/{group_name}/{hook_name}"),
                    name: hook_name.to_owned(),
                    kind: "lifecycle-hook".to_owned(),
                    status: "available".to_owned(),
                    created_at: None,
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes: BTreeMap::from([(
                        "auto_scaling_group".to_owned(),
                        group_name.to_owned(),
                    )]),
                });
            }
        }
    }

    if let Ok(configurations) = client.describe_launch_configurations().send().await {
        for configuration in configurations.launch_configurations() {
            let Some(name) = configuration.launch_configuration_name() else {
                continue;
            };
            let mut attributes = BTreeMap::new();
            insert_attr(&mut attributes, "image_id", configuration.image_id());
            insert_attr(
                &mut attributes,
                "instance_type",
                configuration.instance_type(),
            );

            resources.push(ResourceSummary {
                id: format!("launch-configuration/{name}"),
                name: name.to_owned(),
                kind: "launch-configuration".to_owned(),
                status: "available".to_owned(),
                created_at: super::format_timestamp(configuration.created_time()),
                updated_at: None,
                tags: BTreeMap::new(),
                attributes,
            });
        }
    }

    if let Ok(policies) = client.describe_policies().send().await {
        for policy in policies.scaling_policies() {
            let Some(policy_name) = policy.policy_name() else {
                continue;
            };
            let mut attributes = BTreeMap::new();
            insert_attr(&mut attributes, "policy_arn", policy.policy_arn());
            insert_attr(
                &mut attributes,
                "auto_scaling_group",
                policy.auto_scaling_group_name(),
            );
            insert_attr(&mut attributes, "policy_type", policy.policy_type());

            resources.push(ResourceSummary {
                id: format!("scaling-policy/{policy_name}"),
                name: policy_name.to_owned(),
                kind: "scaling-policy".to_owned(),
                status: "available".to_owned(),
                created_at: None,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes,
            });
        }
    }

    Ok(managed_inventory(
        "autoscaling",
        "Auto Scaling",
        tabs(),
        resources,
        vec![
            "create_auto_scaling_group".to_owned(),
            "delete_policy".to_owned(),
            "delete_lifecycle_hook".to_owned(),
        ],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "update_desired_capacity" => {
            let group_name = require_resource_id(request, "auto-scaling-group/")?;
            let desired_capacity =
                optional_payload_i32(request, "desired_capacity").ok_or_else(|| {
                    ServiceManagementError::invalid_input(
                        request.service_key.clone(),
                        request.action.clone(),
                        "`desired_capacity` is required.",
                    )
                })?;
            client(config)
                .set_desired_capacity()
                .auto_scaling_group_name(group_name)
                .desired_capacity(desired_capacity)
                .honor_cooldown(false)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("autoscaling", "set_desired_capacity", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!(
                    "Updated Auto Scaling group `{group_name}` desired capacity to {desired_capacity}."
                ),
                resource_id: Some(format!("auto-scaling-group/{group_name}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        tab(
            "groups",
            "Groups",
            &["auto-scaling-group"],
            "No Auto Scaling groups were found.",
        ),
        tab(
            "instances",
            "Instances",
            &["scaling-instance"],
            "No Auto Scaling instances are loaded.",
        ),
        tab(
            "policies",
            "Policies",
            &["scaling-policy"],
            "No Auto Scaling policies are loaded.",
        ),
        tab(
            "lifecycle-hooks",
            "Lifecycle Hooks",
            &["lifecycle-hook"],
            "No Auto Scaling lifecycle hooks are loaded.",
        ),
        tab(
            "launch-configurations",
            "Launch Configurations",
            &["launch-configuration"],
            "No launch configurations are loaded.",
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

fn client(config: &AppConfig) -> aws_sdk_autoscaling::Client {
    let sdk_config = aws_sdk_autoscaling::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_autoscaling::config::Region::new(
            config.region.clone(),
        ))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_autoscaling::Client::from_conf(sdk_config)
}

fn tags_to_map(tags: &[aws_sdk_autoscaling::types::TagDescription]) -> BTreeMap<String, String> {
    tags.iter()
        .filter_map(|tag| Some((tag.key()?.to_owned(), tag.value()?.to_owned())))
        .collect()
}
