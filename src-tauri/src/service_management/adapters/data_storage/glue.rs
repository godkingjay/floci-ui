use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        adapters::data_storage::{format_timestamp, local_credentials, read_only_inventory},
        errors::ServiceManagementError,
        models::{ResourceSummary, ResourceTab, ServiceInventory},
    },
};

#[allow(clippy::too_many_lines)]
pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let client = client(config);
    let mut resources = Vec::new();

    let databases = client
        .get_databases()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("glue", "get_databases", err))?;
    for database in databases.database_list() {
        let name = database.name();
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "description", database.description());

        resources.push(ResourceSummary {
            id: format!("database/{name}"),
            name: name.to_owned(),
            kind: "database".to_owned(),
            status: "available".to_owned(),
            created_at: format_timestamp(database.create_time()),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    let tables = client
        .search_tables()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("glue", "search_tables", err))?;
    for table in tables.table_list() {
        let name = table.name();
        let database_name = table.database_name().unwrap_or("unknown");
        let mut attributes = BTreeMap::new();
        attributes.insert("database".to_owned(), database_name.to_owned());
        insert_attr(&mut attributes, "description", table.description());
        insert_attr(&mut attributes, "table_type", table.table_type());
        insert_attr(&mut attributes, "created_by", table.created_by());

        resources.push(ResourceSummary {
            id: format!("table/{database_name}/{name}"),
            name: name.to_owned(),
            kind: "table".to_owned(),
            status: "available".to_owned(),
            created_at: format_timestamp(table.create_time()),
            updated_at: format_timestamp(table.update_time()),
            tags: BTreeMap::new(),
            attributes,
        });
    }

    let crawlers = client
        .get_crawlers()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("glue", "get_crawlers", err))?;
    for crawler in crawlers.crawlers() {
        let Some(name) = crawler.name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "role", crawler.role());
        insert_attr(&mut attributes, "description", crawler.description());

        resources.push(ResourceSummary {
            id: format!("crawler/{name}"),
            name: name.to_owned(),
            kind: "crawler".to_owned(),
            status: crawler
                .state()
                .map_or_else(|| "unknown".to_owned(), ToString::to_string),
            created_at: format_timestamp(crawler.creation_time()),
            updated_at: format_timestamp(crawler.last_updated()),
            tags: BTreeMap::new(),
            attributes,
        });
    }

    let jobs = client
        .get_jobs()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("glue", "get_jobs", err))?;
    for job in jobs.jobs() {
        let Some(name) = job.name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "role", job.role());
        insert_attr(&mut attributes, "description", job.description());

        resources.push(ResourceSummary {
            id: format!("job/{name}"),
            name: name.to_owned(),
            kind: "job".to_owned(),
            status: "defined".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    let connections = client
        .get_connections()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("glue", "get_connections", err))?;
    for connection in connections.connection_list() {
        let Some(name) = connection.name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "description", connection.description());
        insert_attr(
            &mut attributes,
            "connection_type",
            connection.connection_type(),
        );
        insert_attr(
            &mut attributes,
            "last_updated_by",
            connection.last_updated_by(),
        );

        resources.push(ResourceSummary {
            id: format!("connection/{name}"),
            name: name.to_owned(),
            kind: "connection".to_owned(),
            status: "available".to_owned(),
            created_at: format_timestamp(connection.creation_time()),
            updated_at: format_timestamp(connection.last_updated_time()),
            tags: BTreeMap::new(),
            attributes,
        });
    }

    Ok(read_only_inventory("glue", "Glue", tabs(), resources))
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "catalog".to_owned(),
            label: "Catalog".to_owned(),
            kinds: vec![
                "database".to_owned(),
                "table".to_owned(),
                "connection".to_owned(),
            ],
            empty_message: "No Glue catalog databases, tables, or connections were found."
                .to_owned(),
        },
        ResourceTab {
            key: "jobs".to_owned(),
            label: "Jobs".to_owned(),
            kinds: vec!["job".to_owned()],
            empty_message: "No Glue jobs were found.".to_owned(),
        },
        ResourceTab {
            key: "crawlers".to_owned(),
            label: "Crawlers".to_owned(),
            kinds: vec!["crawler".to_owned()],
            empty_message: "No Glue crawlers were found.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_glue::Client {
    let sdk_config = aws_sdk_glue::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_glue::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_glue::Client::from_conf(sdk_config)
}

fn insert_attr(attributes: &mut BTreeMap<String, String>, key: &str, value: Option<impl ToString>) {
    if let Some(value) = value {
        attributes.insert(key.to_owned(), value.to_string());
    }
}
