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
    let streams = client.list_delivery_streams().send().await.map_err(|err| {
        ServiceManagementError::client_error("firehose", "list_delivery_streams", err)
    })?;
    let mut resources = Vec::new();

    for stream_name in streams.delivery_stream_names() {
        let detail = client
            .describe_delivery_stream()
            .delivery_stream_name(stream_name)
            .send()
            .await
            .map_err(|err| {
                ServiceManagementError::client_error("firehose", "describe_delivery_stream", err)
            })?;

        let Some(description) = detail.delivery_stream_description() else {
            continue;
        };

        let name = description.delivery_stream_name();
        let mut attributes = BTreeMap::from([
            (
                "stream_arn".to_owned(),
                description.delivery_stream_arn().to_owned(),
            ),
            (
                "stream_type".to_owned(),
                description.delivery_stream_type().to_string(),
            ),
            ("version".to_owned(), description.version_id().to_owned()),
            (
                "destination_count".to_owned(),
                description.destinations().len().to_string(),
            ),
            (
                "has_more_destinations".to_owned(),
                description.has_more_destinations().to_string(),
            ),
        ]);

        resources.push(ResourceSummary {
            id: format!("delivery-stream/{name}"),
            name: name.to_owned(),
            kind: "delivery-stream".to_owned(),
            status: description.delivery_stream_status().to_string(),
            created_at: format_timestamp(description.create_timestamp()),
            updated_at: format_timestamp(description.last_update_timestamp()),
            tags: BTreeMap::new(),
            attributes: attributes.clone(),
        });

        for destination in description.destinations() {
            let destination_id = destination.destination_id();
            attributes = BTreeMap::from([("stream".to_owned(), name.to_owned())]);
            resources.push(ResourceSummary {
                id: format!("destination/{name}/{destination_id}"),
                name: destination_id.to_owned(),
                kind: "destination".to_owned(),
                status: "configured".to_owned(),
                created_at: None,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes: attributes.clone(),
            });
        }
    }

    Ok(read_only_inventory(
        "firehose",
        "Data Firehose",
        tabs(),
        resources,
    ))
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "streams".to_owned(),
            label: "Streams".to_owned(),
            kinds: vec!["delivery-stream".to_owned()],
            empty_message: "No Firehose delivery streams were found.".to_owned(),
        },
        ResourceTab {
            key: "destinations".to_owned(),
            label: "Destinations".to_owned(),
            kinds: vec!["destination".to_owned()],
            empty_message: "No Firehose destinations are loaded.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_firehose::Client {
    let sdk_config = aws_sdk_firehose::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_firehose::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_firehose::Client::from_conf(sdk_config)
}
