use std::collections::BTreeMap;

use aws_sdk_elasticloadbalancingv2::types::{ProtocolEnum, TargetDescription, TargetTypeEnum};

use crate::{
    config::AppConfig,
    service_management::{
        actions::{require_name_payload, require_typed_confirmation},
        adapters::network_observability::{
            format_timestamp, insert_attr, local_credentials, managed_inventory,
            optional_payload_i32, optional_payload_text, require_payload_text, require_resource_id,
            resource_name_from_arn, unsupported_action,
        },
        errors::ServiceManagementError,
        models::{
            ActionResult, ResourceSummary, ResourceTab, ServiceActionRequest, ServiceInventory,
        },
    },
};

const TARGET_SEPARATOR: &str = "||";

pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let client = client(config);
    let load_balancers = client
        .describe_load_balancers()
        .send()
        .await
        .map_err(|err| {
            ServiceManagementError::client_error("elbv2", "describe_load_balancers", err)
        })?;
    let mut resources = Vec::new();

    for load_balancer in load_balancers.load_balancers() {
        let Some(load_balancer_arn) = load_balancer.load_balancer_arn() else {
            continue;
        };
        let load_balancer_name = load_balancer
            .load_balancer_name()
            .unwrap_or_else(|| resource_name_from_arn(load_balancer_arn));
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "dns_name", load_balancer.dns_name());
        insert_attr(&mut attributes, "vpc_id", load_balancer.vpc_id());
        if let Some(scheme) = load_balancer.scheme() {
            insert_attr(&mut attributes, "scheme", Some(scheme.as_str()));
        }
        if let Some(load_balancer_type) = load_balancer.r#type() {
            insert_attr(&mut attributes, "type", Some(load_balancer_type.as_str()));
        }
        if let Some(state) = load_balancer.state().and_then(|state| state.code()) {
            insert_attr(&mut attributes, "state", Some(state.as_str()));
        }

        resources.push(ResourceSummary {
            id: format!("load-balancer/{load_balancer_arn}"),
            name: load_balancer_name.to_owned(),
            kind: "load-balancer".to_owned(),
            status: load_balancer
                .state()
                .and_then(|state| state.code())
                .map_or_else(|| "available".to_owned(), |state| state.as_str().to_owned()),
            created_at: format_timestamp(load_balancer.created_time()),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });

        load_listeners(&client, load_balancer_arn, &mut resources).await;
    }

    load_target_groups(&client, &mut resources).await;

    Ok(managed_inventory(
        "elbv2",
        "Elastic Load Balancing v2",
        tabs(),
        resources,
        Vec::new(),
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_target_group" => {
            let target_group_name = require_name_payload(request, "target_group_name")?;
            let vpc_id = require_payload_text(request, "vpc_id")?;
            let protocol = ProtocolEnum::from(
                optional_payload_text(request, "protocol")
                    .unwrap_or_else(|| "HTTP".to_owned())
                    .to_ascii_uppercase()
                    .as_str(),
            );
            let port = optional_payload_i32(request, "port").unwrap_or(80);
            let target_type = TargetTypeEnum::from(
                optional_payload_text(request, "target_type")
                    .unwrap_or_else(|| "instance".to_owned())
                    .to_ascii_lowercase()
                    .as_str(),
            );
            let output = client(config)
                .create_target_group()
                .name(&target_group_name)
                .protocol(protocol)
                .port(port)
                .vpc_id(&vpc_id)
                .target_type(target_type)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("elbv2", "create_target_group", err)
                })?;
            let target_group_arn = output
                .target_groups()
                .first()
                .and_then(|target_group| target_group.target_group_arn())
                .map(ToOwned::to_owned);

            Ok(ActionResult {
                changed: true,
                message: format!("Created target group `{target_group_name}`."),
                resource_id: target_group_arn.map(|arn| format!("target-group/{arn}")),
            })
        }
        "register_target" => {
            let target_group_arn = require_resource_id(request, "target-group/")?;
            let target_id = require_payload_text(request, "target_id")?;
            let target = target_description(&target_id, optional_payload_i32(request, "port"));

            client(config)
                .register_targets()
                .target_group_arn(target_group_arn)
                .targets(target)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("elbv2", "register_targets", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Registered target `{target_id}`."),
                resource_id: Some(format!(
                    "target/{target_group_arn}{TARGET_SEPARATOR}{target_id}"
                )),
            })
        }
        "deregister_target" => {
            let target_name = require_typed_confirmation(request)?;
            let (target_group_arn, target_id) = selected_target(request)?;
            let target = target_description(target_id, optional_payload_i32(request, "port"));

            client(config)
                .deregister_targets()
                .target_group_arn(target_group_arn)
                .targets(target)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("elbv2", "deregister_targets", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deregistered target `{target_name}`."),
                resource_id: Some(format!(
                    "target/{target_group_arn}{TARGET_SEPARATOR}{target_id}"
                )),
            })
        }
        "delete_listener" => {
            let listener_name = require_typed_confirmation(request)?;
            let listener_arn = require_resource_id(request, "listener/")?;

            client(config)
                .delete_listener()
                .listener_arn(listener_arn)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("elbv2", "delete_listener", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted listener `{listener_name}`."),
                resource_id: Some(format!("listener/{listener_arn}")),
            })
        }
        "delete_load_balancer" => {
            let load_balancer_name = require_typed_confirmation(request)?;
            let load_balancer_arn = require_resource_id(request, "load-balancer/")?;

            client(config)
                .delete_load_balancer()
                .load_balancer_arn(load_balancer_arn)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("elbv2", "delete_load_balancer", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted load balancer `{load_balancer_name}`."),
                resource_id: Some(format!("load-balancer/{load_balancer_arn}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "load-balancers".to_owned(),
            label: "Load Balancers".to_owned(),
            kinds: vec!["load-balancer".to_owned()],
            empty_message: "No load balancers were found in the local emulator.".to_owned(),
        },
        ResourceTab {
            key: "listeners".to_owned(),
            label: "Listeners".to_owned(),
            kinds: vec!["listener".to_owned()],
            empty_message: "No listeners are loaded.".to_owned(),
        },
        ResourceTab {
            key: "rules".to_owned(),
            label: "Rules".to_owned(),
            kinds: vec!["listener-rule".to_owned()],
            empty_message: "No listener rules are loaded.".to_owned(),
        },
        ResourceTab {
            key: "target-groups".to_owned(),
            label: "Target Groups".to_owned(),
            kinds: vec!["target-group".to_owned()],
            empty_message: "No target groups were found in the local emulator.".to_owned(),
        },
        ResourceTab {
            key: "targets".to_owned(),
            label: "Targets".to_owned(),
            kinds: vec!["target".to_owned()],
            empty_message: "No registered targets are loaded.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_elasticloadbalancingv2::Client {
    let sdk_config = aws_sdk_elasticloadbalancingv2::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_elasticloadbalancingv2::config::Region::new(
            config.region.clone(),
        ))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_elasticloadbalancingv2::Client::from_conf(sdk_config)
}

async fn load_listeners(
    client: &aws_sdk_elasticloadbalancingv2::Client,
    load_balancer_arn: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client
        .describe_listeners()
        .load_balancer_arn(load_balancer_arn)
        .send()
        .await
    else {
        return;
    };

    for listener in output.listeners() {
        let Some(listener_arn) = listener.listener_arn() else {
            continue;
        };
        let name = format!(
            "{}:{}",
            listener
                .protocol()
                .map(|protocol| protocol.as_str())
                .unwrap_or("listener"),
            listener.port().unwrap_or_default()
        );
        let mut attributes = BTreeMap::new();

        insert_attr(
            &mut attributes,
            "load_balancer_arn",
            listener.load_balancer_arn(),
        );
        insert_attr(&mut attributes, "port", listener.port());
        if let Some(protocol) = listener.protocol() {
            insert_attr(&mut attributes, "protocol", Some(protocol.as_str()));
        }

        resources.push(ResourceSummary {
            id: format!("listener/{listener_arn}"),
            name,
            kind: "listener".to_owned(),
            status: listener.protocol().map_or_else(
                || "active".to_owned(),
                |protocol| protocol.as_str().to_owned(),
            ),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });

        load_rules(client, listener_arn, resources).await;
    }
}

async fn load_rules(
    client: &aws_sdk_elasticloadbalancingv2::Client,
    listener_arn: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client
        .describe_rules()
        .listener_arn(listener_arn)
        .send()
        .await
    else {
        return;
    };

    for rule in output.rules() {
        let Some(rule_arn) = rule.rule_arn() else {
            continue;
        };
        let priority = rule.priority().unwrap_or("default");
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "listener_arn", Some(listener_arn));
        insert_attr(&mut attributes, "priority", rule.priority());
        insert_attr(&mut attributes, "is_default", rule.is_default());

        resources.push(ResourceSummary {
            id: format!("listener-rule/{rule_arn}"),
            name: format!("priority {priority}"),
            kind: "listener-rule".to_owned(),
            status: if rule.is_default().unwrap_or_default() {
                "default".to_owned()
            } else {
                "active".to_owned()
            },
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

async fn load_target_groups(
    client: &aws_sdk_elasticloadbalancingv2::Client,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client.describe_target_groups().send().await else {
        return;
    };

    for target_group in output.target_groups() {
        let Some(target_group_arn) = target_group.target_group_arn() else {
            continue;
        };
        let target_group_name = target_group
            .target_group_name()
            .unwrap_or_else(|| resource_name_from_arn(target_group_arn));
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "vpc_id", target_group.vpc_id());
        insert_attr(&mut attributes, "port", target_group.port());
        insert_attr(
            &mut attributes,
            "health_check_port",
            target_group.health_check_port(),
        );
        insert_attr(
            &mut attributes,
            "protocol_version",
            target_group.protocol_version(),
        );
        insert_attr(
            &mut attributes,
            "load_balancer_arns",
            Some(target_group.load_balancer_arns().join(", ")),
        );
        if let Some(protocol) = target_group.protocol() {
            insert_attr(&mut attributes, "protocol", Some(protocol.as_str()));
        }
        if let Some(target_type) = target_group.target_type() {
            insert_attr(&mut attributes, "target_type", Some(target_type.as_str()));
        }

        resources.push(ResourceSummary {
            id: format!("target-group/{target_group_arn}"),
            name: target_group_name.to_owned(),
            kind: "target-group".to_owned(),
            status: target_group.target_type().map_or_else(
                || "available".to_owned(),
                |target_type| target_type.as_str().to_owned(),
            ),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });

        load_targets(client, target_group_arn, resources).await;
    }
}

async fn load_targets(
    client: &aws_sdk_elasticloadbalancingv2::Client,
    target_group_arn: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client
        .describe_target_health()
        .target_group_arn(target_group_arn)
        .send()
        .await
    else {
        return;
    };

    for description in output.target_health_descriptions() {
        let Some(target) = description.target() else {
            continue;
        };
        let Some(target_id) = target.id() else {
            continue;
        };
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "target_group_arn", Some(target_group_arn));
        insert_attr(&mut attributes, "port", target.port());
        insert_attr(
            &mut attributes,
            "health_check_port",
            description.health_check_port(),
        );
        if let Some(health) = description.target_health() {
            if let Some(state) = health.state() {
                insert_attr(&mut attributes, "state", Some(state.as_str()));
            }
            if let Some(reason) = health.reason() {
                insert_attr(&mut attributes, "reason", Some(reason.as_str()));
            }
            insert_attr(&mut attributes, "description", health.description());
        }

        resources.push(ResourceSummary {
            id: format!("target/{target_group_arn}{TARGET_SEPARATOR}{target_id}"),
            name: target_id.to_owned(),
            kind: "target".to_owned(),
            status: description
                .target_health()
                .and_then(|health| health.state())
                .map_or_else(
                    || "registered".to_owned(),
                    |state| state.as_str().to_owned(),
                ),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

fn target_description(target_id: &str, port: Option<i32>) -> TargetDescription {
    let mut builder = TargetDescription::builder().id(target_id);
    if let Some(port) = port {
        builder = builder.port(port);
    }
    builder.build()
}

fn selected_target(request: &ServiceActionRequest) -> Result<(&str, &str), ServiceManagementError> {
    let selected = require_resource_id(request, "target/")?;
    let Some((target_group_arn, target_id)) = selected.split_once(TARGET_SEPARATOR) else {
        return Err(ServiceManagementError::invalid_input(
            request.service_key.clone(),
            request.action.clone(),
            "Select a registered target before running this action.",
        ));
    };

    Ok((target_group_arn, target_id))
}
