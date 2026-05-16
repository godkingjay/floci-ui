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

    let instances =
        client.describe_db_instances().send().await.map_err(|err| {
            ServiceManagementError::client_error("rds", "describe_db_instances", err)
        })?;
    for instance in instances.db_instances() {
        let Some(name) = instance.db_instance_identifier() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "engine", instance.engine());
        insert_attr(&mut attributes, "cluster", instance.db_cluster_identifier());
        insert_attr(&mut attributes, "storage_type", instance.storage_type());
        insert_attr(
            &mut attributes,
            "allocated_storage",
            instance.allocated_storage().map(|value| value.to_string()),
        );

        resources.push(ResourceSummary {
            id: format!("db-instance/{name}"),
            name: name.to_owned(),
            kind: "db-instance".to_owned(),
            status: instance
                .db_instance_status()
                .map_or_else(|| "unknown".to_owned(), ToOwned::to_owned),
            created_at: format_timestamp(instance.instance_create_time()),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    let clusters =
        client.describe_db_clusters().send().await.map_err(|err| {
            ServiceManagementError::client_error("rds", "describe_db_clusters", err)
        })?;
    for cluster in clusters.db_clusters() {
        let Some(name) = cluster.db_cluster_identifier() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "engine", cluster.engine());
        insert_attr(&mut attributes, "engine_version", cluster.engine_version());
        insert_attr(&mut attributes, "endpoint", cluster.endpoint());
        insert_attr(
            &mut attributes,
            "reader_endpoint",
            cluster.reader_endpoint(),
        );

        resources.push(ResourceSummary {
            id: format!("db-cluster/{name}"),
            name: name.to_owned(),
            kind: "db-cluster".to_owned(),
            status: cluster
                .status()
                .map_or_else(|| "unknown".to_owned(), ToOwned::to_owned),
            created_at: format_timestamp(cluster.cluster_create_time()),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    let snapshots =
        client.describe_db_snapshots().send().await.map_err(|err| {
            ServiceManagementError::client_error("rds", "describe_db_snapshots", err)
        })?;
    for snapshot in snapshots.db_snapshots() {
        let Some(name) = snapshot.db_snapshot_identifier() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(
            &mut attributes,
            "instance",
            snapshot.db_instance_identifier(),
        );
        insert_attr(&mut attributes, "engine", snapshot.engine());
        insert_attr(&mut attributes, "snapshot_type", snapshot.snapshot_type());
        insert_attr(
            &mut attributes,
            "allocated_storage",
            snapshot.allocated_storage().map(|value| value.to_string()),
        );

        resources.push(ResourceSummary {
            id: format!("snapshot/{name}"),
            name: name.to_owned(),
            kind: "snapshot".to_owned(),
            status: snapshot
                .status()
                .map_or_else(|| "unknown".to_owned(), ToOwned::to_owned),
            created_at: format_timestamp(snapshot.snapshot_create_time()),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    let subnet_groups = client
        .describe_db_subnet_groups()
        .send()
        .await
        .map_err(|err| {
            ServiceManagementError::client_error("rds", "describe_db_subnet_groups", err)
        })?;
    for subnet_group in subnet_groups.db_subnet_groups() {
        let Some(name) = subnet_group.db_subnet_group_name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(
            &mut attributes,
            "description",
            subnet_group.db_subnet_group_description(),
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
            status: subnet_group
                .subnet_group_status()
                .map_or_else(|| "unknown".to_owned(), ToOwned::to_owned),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    let parameter_groups = client
        .describe_db_parameter_groups()
        .send()
        .await
        .map_err(|err| {
            ServiceManagementError::client_error("rds", "describe_db_parameter_groups", err)
        })?;
    for parameter_group in parameter_groups.db_parameter_groups() {
        let Some(name) = parameter_group.db_parameter_group_name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(
            &mut attributes,
            "family",
            parameter_group.db_parameter_group_family(),
        );
        insert_attr(
            &mut attributes,
            "description",
            parameter_group.description(),
        );

        resources.push(ResourceSummary {
            id: format!("parameter-group/{name}"),
            name: name.to_owned(),
            kind: "parameter-group".to_owned(),
            status: "available".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    Ok(read_only_inventory("rds", "RDS", tabs(), resources))
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "instances".to_owned(),
            label: "Instances".to_owned(),
            kinds: vec!["db-instance".to_owned()],
            empty_message: "No RDS DB instances were found.".to_owned(),
        },
        ResourceTab {
            key: "clusters".to_owned(),
            label: "Clusters".to_owned(),
            kinds: vec!["db-cluster".to_owned()],
            empty_message: "No RDS DB clusters were found.".to_owned(),
        },
        ResourceTab {
            key: "snapshots".to_owned(),
            label: "Snapshots".to_owned(),
            kinds: vec!["snapshot".to_owned()],
            empty_message: "No RDS snapshots were found.".to_owned(),
        },
        ResourceTab {
            key: "subnet-groups".to_owned(),
            label: "Subnet Groups".to_owned(),
            kinds: vec!["subnet-group".to_owned()],
            empty_message: "No RDS subnet groups were found.".to_owned(),
        },
        ResourceTab {
            key: "parameter-groups".to_owned(),
            label: "Parameter Groups".to_owned(),
            kinds: vec!["parameter-group".to_owned()],
            empty_message: "No RDS parameter groups were found.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_rds::Client {
    let sdk_config = aws_sdk_rds::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_rds::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_rds::Client::from_conf(sdk_config)
}

fn insert_attr(attributes: &mut BTreeMap<String, String>, key: &str, value: Option<impl ToString>) {
    if let Some(value) = value {
        attributes.insert(key.to_owned(), value.to_string());
    }
}
