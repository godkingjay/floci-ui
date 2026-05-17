use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        adapters::compute_build::{insert_attr, local_credentials, read_only_inventory},
        errors::ServiceManagementError,
        models::{ResourceSummary, ResourceTab, ServiceInventory},
    },
};

pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let client = client(config);
    let output = client
        .list_clusters_v2()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("msk", "list_clusters_v2", err))?;
    let mut resources = Vec::new();

    for cluster in output.cluster_info_list() {
        let Some(cluster_name) = cluster.cluster_name() else {
            continue;
        };
        let cluster_arn = cluster.cluster_arn().unwrap_or(cluster_name);
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "cluster_arn", Some(cluster_arn));
        insert_attr(&mut attributes, "cluster_type", cluster.cluster_type());

        resources.push(ResourceSummary {
            id: format!("cluster/{cluster_arn}"),
            name: cluster_name.to_owned(),
            kind: "cluster".to_owned(),
            status: cluster
                .state()
                .map_or_else(|| "active".to_owned(), ToString::to_string),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });

        if let Ok(nodes) = client.list_nodes().cluster_arn(cluster_arn).send().await {
            for node in nodes.node_info_list() {
                let node_name = node
                    .broker_node_info()
                    .and_then(|broker| broker.client_subnet())
                    .unwrap_or("broker");
                let mut attributes =
                    BTreeMap::from([("cluster_arn".to_owned(), cluster_arn.to_owned())]);
                if let Some(broker) = node.broker_node_info() {
                    insert_attr(&mut attributes, "broker_id", broker.broker_id());
                    insert_attr(
                        &mut attributes,
                        "client_vpc_ip",
                        broker.client_vpc_ip_address(),
                    );
                    insert_attr(&mut attributes, "subnet", broker.client_subnet());
                }

                resources.push(ResourceSummary {
                    id: format!("broker/{cluster_arn}/{node_name}"),
                    name: node_name.to_owned(),
                    kind: "broker".to_owned(),
                    status: "available".to_owned(),
                    created_at: None,
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes,
                });
            }
        }
    }

    if let Ok(configurations) = client.list_configurations().send().await {
        for configuration in configurations.configurations() {
            let Some(arn) = configuration.arn() else {
                continue;
            };
            resources.push(ResourceSummary {
                id: format!("configuration/{arn}"),
                name: configuration.name().unwrap_or(arn).to_owned(),
                kind: "configuration".to_owned(),
                status: "available".to_owned(),
                created_at: None,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes: BTreeMap::from([("arn".to_owned(), arn.to_owned())]),
            });
        }
    }

    Ok(read_only_inventory("msk", "MSK", tabs(), resources))
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        tab(
            "clusters",
            "Clusters",
            &["cluster"],
            "No MSK clusters were found.",
        ),
        tab(
            "brokers",
            "Brokers",
            &["broker"],
            "No MSK brokers are loaded.",
        ),
        tab(
            "configurations",
            "Configurations",
            &["configuration"],
            "No MSK configurations are loaded.",
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

fn client(config: &AppConfig) -> aws_sdk_kafka::Client {
    let sdk_config = aws_sdk_kafka::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_kafka::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_kafka::Client::from_conf(sdk_config)
}
