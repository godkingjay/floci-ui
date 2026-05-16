use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        adapters::data_storage::{format_timestamp, local_credentials, read_only_inventory},
        errors::ServiceManagementError,
        models::{ResourceSummary, ResourceTab, ServiceInventory},
    },
};

pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let client = client(config);
    let mut resources = Vec::new();

    let clusters = client
        .describe_cache_clusters()
        .send()
        .await
        .map_err(|err| {
            ServiceManagementError::client_error("elasticache", "describe_cache_clusters", err)
        })?;
    for cluster in clusters.cache_clusters() {
        let Some(name) = cluster.cache_cluster_id() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "engine", cluster.engine());
        insert_attr(
            &mut attributes,
            "subnet_group",
            cluster.cache_subnet_group_name(),
        );
        insert_attr(
            &mut attributes,
            "replication_group",
            cluster.replication_group_id(),
        );

        resources.push(ResourceSummary {
            id: format!("cache-cluster/{name}"),
            name: name.to_owned(),
            kind: "cache-cluster".to_owned(),
            status: cluster
                .cache_cluster_status()
                .map_or_else(|| "unknown".to_owned(), ToOwned::to_owned),
            created_at: format_timestamp(cluster.cache_cluster_create_time()),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    let replication_groups = client
        .describe_replication_groups()
        .send()
        .await
        .map_err(|err| {
            ServiceManagementError::client_error("elasticache", "describe_replication_groups", err)
        })?;
    for group in replication_groups.replication_groups() {
        let Some(name) = group.replication_group_id() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "engine", group.engine());
        insert_attr(&mut attributes, "node_type", group.cache_node_type());
        attributes.insert(
            "member_cluster_count".to_owned(),
            group.member_clusters().len().to_string(),
        );
        attributes.insert(
            "node_group_count".to_owned(),
            group.node_groups().len().to_string(),
        );

        resources.push(ResourceSummary {
            id: format!("replication-group/{name}"),
            name: name.to_owned(),
            kind: "replication-group".to_owned(),
            status: group
                .status()
                .map_or_else(|| "unknown".to_owned(), ToOwned::to_owned),
            created_at: format_timestamp(group.replication_group_create_time()),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    let users = client.describe_users().send().await.map_err(|err| {
        ServiceManagementError::client_error("elasticache", "describe_users", err)
    })?;
    for user in users.users() {
        let Some(user_id) = user.user_id() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "user_name", user.user_name());
        insert_attr(&mut attributes, "engine", user.engine());
        insert_attr(&mut attributes, "access", user.access_string());

        resources.push(ResourceSummary {
            id: format!("user/{user_id}"),
            name: user_id.to_owned(),
            kind: "user".to_owned(),
            status: user
                .status()
                .map_or_else(|| "unknown".to_owned(), ToOwned::to_owned),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    let subnet_groups = client
        .describe_cache_subnet_groups()
        .send()
        .await
        .map_err(|err| {
            ServiceManagementError::client_error("elasticache", "describe_cache_subnet_groups", err)
        })?;
    for subnet_group in subnet_groups.cache_subnet_groups() {
        let Some(name) = subnet_group.cache_subnet_group_name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(
            &mut attributes,
            "description",
            subnet_group.cache_subnet_group_description(),
        );
        insert_attr(&mut attributes, "vpc", subnet_group.vpc_id());
        attributes.insert(
            "subnet_count".to_owned(),
            subnet_group.subnets().len().to_string(),
        );

        resources.push(ResourceSummary {
            id: format!("subnet-group/{name}"),
            name: name.to_owned(),
            kind: "subnet-group".to_owned(),
            status: "available".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    Ok(read_only_inventory(
        "elasticache",
        "ElastiCache",
        tabs(),
        resources,
    ))
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "clusters".to_owned(),
            label: "Clusters".to_owned(),
            kinds: vec!["cache-cluster".to_owned()],
            empty_message: "No ElastiCache clusters were found.".to_owned(),
        },
        ResourceTab {
            key: "replication-groups".to_owned(),
            label: "Replication Groups".to_owned(),
            kinds: vec!["replication-group".to_owned()],
            empty_message: "No ElastiCache replication groups were found.".to_owned(),
        },
        ResourceTab {
            key: "users".to_owned(),
            label: "Users".to_owned(),
            kinds: vec!["user".to_owned()],
            empty_message: "No ElastiCache users were found.".to_owned(),
        },
        ResourceTab {
            key: "subnet-groups".to_owned(),
            label: "Subnet Groups".to_owned(),
            kinds: vec!["subnet-group".to_owned()],
            empty_message: "No ElastiCache subnet groups were found.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_elasticache::Client {
    let sdk_config = aws_sdk_elasticache::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_elasticache::config::Region::new(
            config.region.clone(),
        ))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_elasticache::Client::from_conf(sdk_config)
}

fn insert_attr(attributes: &mut BTreeMap<String, String>, key: &str, value: Option<impl ToString>) {
    if let Some(value) = value {
        attributes.insert(key.to_owned(), value.to_string());
    }
}
