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
    let output = client.list_domain_names().send().await.map_err(|err| {
        ServiceManagementError::client_error("opensearch", "list_domain_names", err)
    })?;
    let domain_names = output
        .domain_names()
        .iter()
        .filter_map(|domain| domain.domain_name().map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    let mut resources = Vec::new();

    if domain_names.is_empty() {
        return Ok(read_only_inventory(
            "opensearch",
            "OpenSearch",
            tabs(),
            resources,
        ));
    }

    let mut describe = client.describe_domains();
    for domain_name in &domain_names {
        describe = describe.domain_names(domain_name.clone());
    }

    let details = describe.send().await.map_err(|err| {
        ServiceManagementError::client_error("opensearch", "describe_domains", err)
    })?;

    for domain in details.domain_status_list() {
        let name = domain.domain_name();
        let mut attributes = BTreeMap::new();
        attributes.insert("domain_id".to_owned(), domain.domain_id().to_owned());
        attributes.insert("arn".to_owned(), domain.arn().to_owned());
        insert_attr(&mut attributes, "engine_version", domain.engine_version());
        insert_attr(&mut attributes, "endpoint", domain.endpoint());
        insert_attr(&mut attributes, "endpoint_v2", domain.endpoint_v2());
        if let Some(endpoints) = domain.endpoints() {
            attributes.insert("endpoint_count".to_owned(), endpoints.len().to_string());
        }

        resources.push(ResourceSummary {
            id: format!("domain/{name}"),
            name: name.to_owned(),
            kind: "domain".to_owned(),
            status: domain_status(domain.created(), domain.deleted(), domain.processing()),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });

        if let Some(endpoint) = domain.endpoint() {
            resources.push(endpoint_resource(name, "primary", endpoint));
        }
        if let Some(endpoint) = domain.endpoint_v2() {
            resources.push(endpoint_resource(name, "v2", endpoint));
        }
        if let Some(endpoints) = domain.endpoints() {
            for (endpoint_type, endpoint) in endpoints {
                resources.push(endpoint_resource(name, endpoint_type, endpoint));
            }
        }
    }

    Ok(read_only_inventory(
        "opensearch",
        "OpenSearch",
        tabs(),
        resources,
    ))
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "domains".to_owned(),
            label: "Domains".to_owned(),
            kinds: vec!["domain".to_owned()],
            empty_message: "No OpenSearch domains were found.".to_owned(),
        },
        ResourceTab {
            key: "endpoints".to_owned(),
            label: "Endpoints".to_owned(),
            kinds: vec!["endpoint".to_owned()],
            empty_message: "No OpenSearch endpoints are loaded.".to_owned(),
        },
        ResourceTab {
            key: "index-hints".to_owned(),
            label: "Index Hints".to_owned(),
            kinds: vec!["index-hint".to_owned()],
            empty_message: "No OpenSearch index hints are loaded.".to_owned(),
        },
        ResourceTab {
            key: "access-policies".to_owned(),
            label: "Access Policies".to_owned(),
            kinds: vec!["access-policy".to_owned()],
            empty_message: "No OpenSearch access policies are loaded.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_opensearch::Client {
    let sdk_config = aws_sdk_opensearch::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_opensearch::config::Region::new(
            config.region.clone(),
        ))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_opensearch::Client::from_conf(sdk_config)
}

fn endpoint_resource(domain_name: &str, endpoint_type: &str, endpoint: &str) -> ResourceSummary {
    ResourceSummary {
        id: format!("endpoint/{domain_name}/{endpoint_type}"),
        name: endpoint.to_owned(),
        kind: "endpoint".to_owned(),
        status: "available".to_owned(),
        created_at: None,
        updated_at: None,
        tags: BTreeMap::new(),
        attributes: BTreeMap::from([
            ("domain".to_owned(), domain_name.to_owned()),
            ("endpoint_type".to_owned(), endpoint_type.to_owned()),
        ]),
    }
}

fn domain_status(created: Option<bool>, deleted: Option<bool>, processing: Option<bool>) -> String {
    if deleted.unwrap_or_default() {
        "deleted".to_owned()
    } else if processing.unwrap_or_default() {
        "processing".to_owned()
    } else if created.unwrap_or_default() {
        "active".to_owned()
    } else {
        "listed".to_owned()
    }
}

fn insert_attr(attributes: &mut BTreeMap<String, String>, key: &str, value: Option<impl ToString>) {
    if let Some(value) = value {
        attributes.insert(key.to_owned(), value.to_string());
    }
}
