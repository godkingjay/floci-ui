use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        adapters::data_storage::{local_credentials, read_only_inventory},
        errors::ServiceManagementError,
        models::{ResourceSummary, ResourceTab, ServiceInventory},
    },
};

pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let client = client(config);
    let mut resources = Vec::new();

    let workgroups =
        client.list_work_groups().send().await.map_err(|err| {
            ServiceManagementError::client_error("athena", "list_work_groups", err)
        })?;
    for workgroup in workgroups.work_groups() {
        let Some(name) = workgroup.name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "description", workgroup.description());

        resources.push(ResourceSummary {
            id: format!("workgroup/{name}"),
            name: name.to_owned(),
            kind: "workgroup".to_owned(),
            status: workgroup
                .state()
                .map_or_else(|| "unknown".to_owned(), ToString::to_string),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    let catalogs =
        client.list_data_catalogs().send().await.map_err(|err| {
            ServiceManagementError::client_error("athena", "list_data_catalogs", err)
        })?;
    for catalog in catalogs.data_catalogs_summary() {
        let Some(name) = catalog.catalog_name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "catalog_type", catalog.r#type());
        insert_attr(
            &mut attributes,
            "connection_type",
            catalog.connection_type(),
        );
        insert_attr(&mut attributes, "error", catalog.error());

        resources.push(ResourceSummary {
            id: format!("data-catalog/{name}"),
            name: name.to_owned(),
            kind: "data-catalog".to_owned(),
            status: catalog
                .status()
                .map_or_else(|| "available".to_owned(), ToString::to_string),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    let query_ids = client
        .list_named_queries()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("athena", "list_named_queries", err))?
        .named_query_ids()
        .to_vec();
    if !query_ids.is_empty() {
        let mut batch = client.batch_get_named_query();
        for query_id in &query_ids {
            batch = batch.named_query_ids(query_id.clone());
        }
        let queries = batch.send().await.map_err(|err| {
            ServiceManagementError::client_error("athena", "batch_get_named_query", err)
        })?;
        for query in queries.named_queries() {
            let name = query.name();
            let query_id = query.named_query_id().unwrap_or(name);
            let mut attributes = BTreeMap::new();
            insert_attr(&mut attributes, "description", query.description());
            attributes.insert("database".to_owned(), query.database().to_owned());
            insert_attr(&mut attributes, "workgroup", query.work_group());

            resources.push(ResourceSummary {
                id: format!("named-query/{query_id}"),
                name: name.to_owned(),
                kind: "named-query".to_owned(),
                status: "saved".to_owned(),
                created_at: None,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes,
            });
        }
    }

    Ok(read_only_inventory("athena", "Athena", tabs(), resources))
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "workgroups".to_owned(),
            label: "Workgroups".to_owned(),
            kinds: vec!["workgroup".to_owned()],
            empty_message: "No Athena workgroups were found.".to_owned(),
        },
        ResourceTab {
            key: "queries".to_owned(),
            label: "Queries".to_owned(),
            kinds: vec!["named-query".to_owned()],
            empty_message: "No Athena named queries were found.".to_owned(),
        },
        ResourceTab {
            key: "results-metadata".to_owned(),
            label: "Results Metadata".to_owned(),
            kinds: vec!["result-metadata".to_owned()],
            empty_message: "No Athena result metadata is loaded.".to_owned(),
        },
        ResourceTab {
            key: "catalog".to_owned(),
            label: "Catalog".to_owned(),
            kinds: vec!["data-catalog".to_owned()],
            empty_message: "No Athena data catalogs were found.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_athena::Client {
    let sdk_config = aws_sdk_athena::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_athena::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_athena::Client::from_conf(sdk_config)
}

fn insert_attr(attributes: &mut BTreeMap<String, String>, key: &str, value: Option<impl ToString>) {
    if let Some(value) = value {
        attributes.insert(key.to_owned(), value.to_string());
    }
}
