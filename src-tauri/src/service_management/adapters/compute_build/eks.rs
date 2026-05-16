use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        adapters::compute_build::{
            format_timestamp, insert_attr, local_credentials, read_only_inventory,
        },
        errors::ServiceManagementError,
        models::{ResourceSummary, ResourceTab, ServiceInventory},
    },
};

pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let client = client(config);
    let output = client
        .list_clusters()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("eks", "list_clusters", err))?;
    let mut resources = Vec::new();

    for cluster_name in output.clusters() {
        if let Ok(cluster_output) = client.describe_cluster().name(cluster_name).send().await {
            if let Some(cluster) = cluster_output.cluster() {
                let mut attributes = BTreeMap::new();
                insert_attr(&mut attributes, "arn", cluster.arn());
                insert_attr(&mut attributes, "endpoint", cluster.endpoint());
                insert_attr(&mut attributes, "version", cluster.version());
                insert_attr(
                    &mut attributes,
                    "platform_version",
                    cluster.platform_version(),
                );

                resources.push(ResourceSummary {
                    id: format!("cluster/{cluster_name}"),
                    name: cluster_name.to_owned(),
                    kind: "cluster".to_owned(),
                    status: cluster
                        .status()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "active".to_owned()),
                    created_at: format_timestamp(cluster.created_at()),
                    updated_at: None,
                    tags: cluster
                        .tags()
                        .map(|tags| {
                            tags.iter()
                                .map(|(key, value)| (key.clone(), value.clone()))
                                .collect()
                        })
                        .unwrap_or_default(),
                    attributes,
                });
            }
        }

        if let Ok(nodegroups) = client
            .list_nodegroups()
            .cluster_name(cluster_name)
            .send()
            .await
        {
            for nodegroup_name in nodegroups.nodegroups() {
                let mut attributes =
                    BTreeMap::from([("cluster_name".to_owned(), cluster_name.to_owned())]);
                if let Ok(nodegroup_output) = client
                    .describe_nodegroup()
                    .cluster_name(cluster_name)
                    .nodegroup_name(nodegroup_name)
                    .send()
                    .await
                {
                    if let Some(nodegroup) = nodegroup_output.nodegroup() {
                        insert_attr(&mut attributes, "nodegroup_arn", nodegroup.nodegroup_arn());
                        insert_attr(&mut attributes, "capacity_type", nodegroup.capacity_type());
                    }
                }

                resources.push(ResourceSummary {
                    id: format!("node-group/{cluster_name}/{nodegroup_name}"),
                    name: nodegroup_name.to_owned(),
                    kind: "node-group".to_owned(),
                    status: "available".to_owned(),
                    created_at: None,
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes,
                });
            }
        }

        if let Ok(addons) = client.list_addons().cluster_name(cluster_name).send().await {
            for addon_name in addons.addons() {
                resources.push(ResourceSummary {
                    id: format!("addon/{cluster_name}/{addon_name}"),
                    name: addon_name.to_owned(),
                    kind: "addon".to_owned(),
                    status: "available".to_owned(),
                    created_at: None,
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes: BTreeMap::from([(
                        "cluster_name".to_owned(),
                        cluster_name.to_owned(),
                    )]),
                });
            }
        }
    }

    Ok(read_only_inventory("eks", "EKS", tabs(), resources))
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        tab(
            "clusters",
            "Clusters",
            &["cluster"],
            "No EKS clusters were found.",
        ),
        tab(
            "node-groups",
            "Node Groups",
            &["node-group"],
            "No EKS node groups are loaded.",
        ),
        tab(
            "addons",
            "Add-ons",
            &["addon"],
            "No EKS add-ons are loaded.",
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

fn client(config: &AppConfig) -> aws_sdk_eks::Client {
    let sdk_config = aws_sdk_eks::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_eks::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_eks::Client::from_conf(sdk_config)
}
