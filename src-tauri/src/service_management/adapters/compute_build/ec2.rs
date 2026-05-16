use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        actions::require_typed_confirmation,
        adapters::compute_build::{
            format_timestamp, insert_attr, local_credentials, managed_inventory,
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
    let mut resources = Vec::new();

    let instances =
        client.describe_instances().send().await.map_err(|err| {
            ServiceManagementError::client_error("ec2", "describe_instances", err)
        })?;
    for reservation in instances.reservations() {
        for instance in reservation.instances() {
            let Some(instance_id) = instance.instance_id() else {
                continue;
            };
            let mut attributes = BTreeMap::new();
            insert_attr(&mut attributes, "instance_type", instance.instance_type());
            insert_attr(&mut attributes, "private_ip", instance.private_ip_address());
            insert_attr(&mut attributes, "public_ip", instance.public_ip_address());
            insert_attr(&mut attributes, "vpc_id", instance.vpc_id());
            insert_attr(&mut attributes, "subnet_id", instance.subnet_id());

            resources.push(ResourceSummary {
                id: format!("instance/{instance_id}"),
                name: instance_name(instance_id, instance.tags()),
                kind: "instance".to_owned(),
                status: instance
                    .state()
                    .and_then(|state| state.name())
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "unknown".to_owned()),
                created_at: format_timestamp(instance.launch_time()),
                updated_at: None,
                tags: tags_to_map(instance.tags()),
                attributes,
            });
        }
    }

    if let Ok(vpcs) = client.describe_vpcs().send().await {
        for vpc in vpcs.vpcs() {
            let Some(vpc_id) = vpc.vpc_id() else {
                continue;
            };
            resources.push(ResourceSummary {
                id: format!("vpc/{vpc_id}"),
                name: vpc_id.to_owned(),
                kind: "vpc".to_owned(),
                status: vpc
                    .state()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "available".to_owned()),
                created_at: None,
                updated_at: None,
                tags: tags_to_map(vpc.tags()),
                attributes: BTreeMap::from([(
                    "cidr_block".to_owned(),
                    vpc.cidr_block().unwrap_or("n/a").to_owned(),
                )]),
            });
        }
    }

    if let Ok(subnets) = client.describe_subnets().send().await {
        for subnet in subnets.subnets() {
            let Some(subnet_id) = subnet.subnet_id() else {
                continue;
            };
            let mut attributes = BTreeMap::new();
            insert_attr(&mut attributes, "vpc_id", subnet.vpc_id());
            insert_attr(&mut attributes, "cidr_block", subnet.cidr_block());
            insert_attr(
                &mut attributes,
                "availability_zone",
                subnet.availability_zone(),
            );
            insert_attr(
                &mut attributes,
                "available_ip_count",
                subnet.available_ip_address_count(),
            );

            resources.push(ResourceSummary {
                id: format!("subnet/{subnet_id}"),
                name: subnet_id.to_owned(),
                kind: "subnet".to_owned(),
                status: subnet
                    .state()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "available".to_owned()),
                created_at: None,
                updated_at: None,
                tags: tags_to_map(subnet.tags()),
                attributes,
            });
        }
    }

    if let Ok(groups) = client.describe_security_groups().send().await {
        for group in groups.security_groups() {
            let Some(group_id) = group.group_id() else {
                continue;
            };
            let mut attributes = BTreeMap::new();
            insert_attr(&mut attributes, "group_name", group.group_name());
            insert_attr(&mut attributes, "description", group.description());
            insert_attr(&mut attributes, "vpc_id", group.vpc_id());
            insert_attr(
                &mut attributes,
                "inbound_rules",
                Some(group.ip_permissions().len()),
            );

            resources.push(ResourceSummary {
                id: format!("security-group/{group_id}"),
                name: group.group_name().unwrap_or(group_id).to_owned(),
                kind: "security-group".to_owned(),
                status: "available".to_owned(),
                created_at: None,
                updated_at: None,
                tags: tags_to_map(group.tags()),
                attributes,
            });
        }
    }

    if let Ok(key_pairs) = client.describe_key_pairs().send().await {
        for key_pair in key_pairs.key_pairs() {
            let Some(key_name) = key_pair.key_name() else {
                continue;
            };
            let mut attributes = BTreeMap::new();
            insert_attr(&mut attributes, "key_pair_id", key_pair.key_pair_id());
            insert_attr(&mut attributes, "fingerprint", key_pair.key_fingerprint());

            resources.push(ResourceSummary {
                id: format!("key-pair/{key_name}"),
                name: key_name.to_owned(),
                kind: "key-pair".to_owned(),
                status: "available".to_owned(),
                created_at: None,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes,
            });
        }
    }

    if let Ok(images) = client.describe_images().owners("self").send().await {
        for image in images.images() {
            let Some(image_id) = image.image_id() else {
                continue;
            };
            let mut attributes = BTreeMap::new();
            insert_attr(&mut attributes, "architecture", image.architecture());
            insert_attr(&mut attributes, "image_type", image.image_type());
            insert_attr(
                &mut attributes,
                "root_device_type",
                image.root_device_type(),
            );

            resources.push(ResourceSummary {
                id: format!("image/{image_id}"),
                name: image.name().unwrap_or(image_id).to_owned(),
                kind: "image".to_owned(),
                status: image
                    .state()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "available".to_owned()),
                created_at: image.creation_date().map(ToOwned::to_owned),
                updated_at: None,
                tags: tags_to_map(image.tags()),
                attributes,
            });
        }
    }

    if let Ok(volumes) = client.describe_volumes().send().await {
        for volume in volumes.volumes() {
            let Some(volume_id) = volume.volume_id() else {
                continue;
            };
            let mut attributes = BTreeMap::new();
            insert_attr(&mut attributes, "size_gib", volume.size());
            insert_attr(&mut attributes, "volume_type", volume.volume_type());
            insert_attr(
                &mut attributes,
                "availability_zone",
                volume.availability_zone(),
            );

            resources.push(ResourceSummary {
                id: format!("volume/{volume_id}"),
                name: volume_id.to_owned(),
                kind: "volume".to_owned(),
                status: volume
                    .state()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "available".to_owned()),
                created_at: format_timestamp(volume.create_time()),
                updated_at: None,
                tags: tags_to_map(volume.tags()),
                attributes,
            });
        }
    }

    Ok(managed_inventory(
        "ec2",
        "EC2",
        tabs(),
        resources,
        vec![
            "create_instance".to_owned(),
            "modify_instance".to_owned(),
            "delete_security_group".to_owned(),
        ],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "start_instance" => {
            let instance_id = require_resource_id(request, "instance/")?;
            client(config)
                .start_instances()
                .instance_ids(instance_id)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("ec2", "start_instances", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Started EC2 instance `{instance_id}`."),
                resource_id: Some(format!("instance/{instance_id}")),
            })
        }
        "stop_instance" => {
            let instance_id = require_typed_confirmation(request)?;
            client(config)
                .stop_instances()
                .instance_ids(&instance_id)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("ec2", "stop_instances", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Stopped EC2 instance `{instance_id}`."),
                resource_id: Some(format!("instance/{instance_id}")),
            })
        }
        "terminate_instance" => {
            let instance_id = require_typed_confirmation(request)?;
            client(config)
                .terminate_instances()
                .instance_ids(&instance_id)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("ec2", "terminate_instances", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Terminated EC2 instance `{instance_id}`."),
                resource_id: Some(format!("instance/{instance_id}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        tab(
            "instances",
            "Instances",
            &["instance"],
            "No EC2 instances were found.",
        ),
        tab("vpcs", "VPCs", &["vpc"], "No EC2 VPCs are loaded."),
        tab(
            "subnets",
            "Subnets",
            &["subnet"],
            "No EC2 subnets are loaded.",
        ),
        tab(
            "security-groups",
            "Security Groups",
            &["security-group"],
            "No EC2 security groups are loaded.",
        ),
        tab(
            "key-pairs",
            "Key Pairs",
            &["key-pair"],
            "No EC2 key pairs are loaded.",
        ),
        tab(
            "images",
            "Images",
            &["image"],
            "No local EC2 images are loaded.",
        ),
        tab(
            "volumes",
            "Volumes",
            &["volume"],
            "No EC2 volumes are loaded.",
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

fn client(config: &AppConfig) -> aws_sdk_ec2::Client {
    let sdk_config = aws_sdk_ec2::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_ec2::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_ec2::Client::from_conf(sdk_config)
}

fn tags_to_map(tags: &[aws_sdk_ec2::types::Tag]) -> BTreeMap<String, String> {
    tags.iter()
        .filter_map(|tag| Some((tag.key()?.to_owned(), tag.value()?.to_owned())))
        .collect()
}

fn instance_name(instance_id: &str, tags: &[aws_sdk_ec2::types::Tag]) -> String {
    tags.iter()
        .find_map(|tag| {
            (tag.key() == Some("Name"))
                .then(|| tag.value().map(ToOwned::to_owned))
                .flatten()
        })
        .unwrap_or_else(|| instance_id.to_owned())
}
