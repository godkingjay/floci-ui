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
    let output = client.list_streams().send().await.map_err(|err| {
        ServiceManagementError::client_error("dynamodbstreams", "list_streams", err)
    })?;
    let mut resources = Vec::new();

    for stream in output.streams() {
        let Some(stream_arn) = stream.stream_arn() else {
            continue;
        };

        let mut name = stream
            .stream_label()
            .map_or_else(|| stream_arn.to_owned(), ToOwned::to_owned);
        let mut status = "listed".to_owned();
        let mut created_at = None;
        let table_name = stream.table_name().map(ToOwned::to_owned);

        if let Ok(detail) = client.describe_stream().stream_arn(stream_arn).send().await {
            if let Some(description) = detail.stream_description() {
                if let Some(label) = description.stream_label() {
                    name = label.to_owned();
                }
                status = description
                    .stream_status()
                    .map_or_else(|| "unknown".to_owned(), ToString::to_string);
                created_at = format_timestamp(description.creation_request_date_time());
            }
        }

        let mut attributes = BTreeMap::from([("stream_arn".to_owned(), stream_arn.to_owned())]);
        if let Some(table_name) = table_name {
            attributes.insert("table".to_owned(), table_name);
        }

        resources.push(ResourceSummary {
            id: format!("stream/{stream_arn}"),
            name,
            kind: "stream".to_owned(),
            status,
            created_at,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    Ok(read_only_inventory(
        "dynamodbstreams",
        "DynamoDB Streams",
        tabs(),
        resources,
    ))
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![ResourceTab {
        key: "streams".to_owned(),
        label: "Streams".to_owned(),
        kinds: vec!["stream".to_owned()],
        empty_message: "No DynamoDB stream records are exposed by the local endpoint.".to_owned(),
    }]
}

fn client(config: &AppConfig) -> aws_sdk_dynamodbstreams::Client {
    let sdk_config = aws_sdk_dynamodbstreams::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_dynamodbstreams::config::Region::new(
            config.region.clone(),
        ))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_dynamodbstreams::Client::from_conf(sdk_config)
}
