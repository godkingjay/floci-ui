use std::collections::BTreeMap;

use aws_sdk_kinesis::primitives::Blob;

use crate::{
    config::AppConfig,
    service_management::{
        actions::{require_name_payload, require_typed_confirmation},
        adapters::messaging_events::{
            format_timestamp, insert_attr, local_credentials, managed_inventory,
            require_payload_text, require_resource_id, unsupported_action,
        },
        errors::ServiceManagementError,
        models::{
            ActionResult, ResourceSummary, ResourceTab, ServiceActionRequest, ServiceInventory,
        },
    },
};

#[allow(clippy::too_many_lines)]
pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let client = client(config);
    let output = client
        .list_streams()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("kinesis", "list_streams", err))?;
    let mut resources = Vec::new();

    for stream_name in output.stream_names() {
        let detail = client
            .describe_stream_summary()
            .stream_name(stream_name)
            .send()
            .await
            .map_err(|err| {
                ServiceManagementError::client_error("kinesis", "describe_stream_summary", err)
            })?;
        let Some(summary) = detail.stream_description_summary() else {
            continue;
        };
        let stream_arn = summary.stream_arn().to_owned();
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "stream_arn", Some(stream_arn.clone()));
        insert_attr(
            &mut attributes,
            "retention_hours",
            Some(summary.retention_period_hours()),
        );
        insert_attr(
            &mut attributes,
            "open_shards",
            Some(summary.open_shard_count()),
        );
        if let Some(mode) = summary.stream_mode_details() {
            insert_attr(&mut attributes, "stream_mode", Some(mode.stream_mode()));
        }

        resources.push(ResourceSummary {
            id: format!("stream/{stream_name}"),
            name: stream_name.to_owned(),
            kind: "stream".to_owned(),
            status: summary.stream_status().to_string(),
            created_at: format_timestamp(Some(summary.stream_creation_timestamp())),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });

        resources.push(ResourceSummary {
            id: format!("metrics/{stream_name}"),
            name: format!("{stream_name} metrics"),
            kind: "metric".to_owned(),
            status: "available".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes: BTreeMap::from([
                (
                    "open_shards".to_owned(),
                    summary.open_shard_count().to_string(),
                ),
                (
                    "retention_hours".to_owned(),
                    summary.retention_period_hours().to_string(),
                ),
            ]),
        });

        if let Ok(shards) = client.list_shards().stream_name(stream_name).send().await {
            for shard in shards.shards() {
                let mut attributes =
                    BTreeMap::from([("stream_name".to_owned(), stream_name.to_owned())]);
                insert_attr(&mut attributes, "parent_shard_id", shard.parent_shard_id());
                insert_attr(
                    &mut attributes,
                    "adjacent_parent_shard_id",
                    shard.adjacent_parent_shard_id(),
                );

                resources.push(ResourceSummary {
                    id: format!("shard/{stream_name}/{}", shard.shard_id()),
                    name: shard.shard_id().to_owned(),
                    kind: "shard".to_owned(),
                    status: "open".to_owned(),
                    created_at: None,
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes,
                });
            }
        }

        if let Ok(consumers) = client
            .list_stream_consumers()
            .stream_arn(stream_arn.clone())
            .send()
            .await
        {
            for consumer in consumers.consumers() {
                let attributes = BTreeMap::from([
                    ("stream_arn".to_owned(), stream_arn.clone()),
                    (
                        "consumer_arn".to_owned(),
                        consumer.consumer_arn().to_owned(),
                    ),
                ]);

                resources.push(ResourceSummary {
                    id: format!("consumer/{}", consumer.consumer_arn()),
                    name: consumer.consumer_name().to_owned(),
                    kind: "consumer".to_owned(),
                    status: consumer.consumer_status().to_string(),
                    created_at: format_timestamp(Some(consumer.consumer_creation_timestamp())),
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes,
                });
            }
        }
    }

    Ok(managed_inventory(
        "kinesis",
        "Kinesis",
        tabs(),
        resources,
        vec!["register_consumer".to_owned(), "delete_consumer".to_owned()],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_stream" => {
            let stream_name = require_name_payload(request, "stream_name")?;
            client(config)
                .create_stream()
                .stream_name(&stream_name)
                .shard_count(1)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("kinesis", "create_stream", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created Kinesis stream `{stream_name}`."),
                resource_id: Some(format!("stream/{stream_name}")),
            })
        }
        "delete_stream" => {
            let stream_name = require_typed_confirmation(request)?;
            let stream_id = require_resource_id(request, "stream/")?;
            client(config)
                .delete_stream()
                .stream_name(stream_id)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("kinesis", "delete_stream", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted Kinesis stream `{stream_name}`."),
                resource_id: Some(format!("stream/{stream_id}")),
            })
        }
        "put_record" => {
            let stream_name = require_resource_id(request, "stream/")?;
            let partition_key = require_payload_text(request, "partition_key")?;
            let data = require_payload_text(request, "data")?;
            let output = client(config)
                .put_record()
                .stream_name(stream_name)
                .partition_key(&partition_key)
                .data(Blob::new(data.into_bytes()))
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("kinesis", "put_record", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Put Kinesis record on shard `{}`.", output.shard_id()),
                resource_id: Some(format!("stream/{stream_name}")),
            })
        }
        "refresh_stream_summary" => {
            let stream_name = require_resource_id(request, "stream/")?;
            Ok(ActionResult {
                changed: false,
                message: "Stream summary refresh completed.".to_owned(),
                resource_id: Some(format!("stream/{stream_name}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "streams".to_owned(),
            label: "Streams".to_owned(),
            kinds: vec!["stream".to_owned()],
            empty_message: "No Kinesis streams were found in the local emulator.".to_owned(),
        },
        ResourceTab {
            key: "shards".to_owned(),
            label: "Shards".to_owned(),
            kinds: vec!["shard".to_owned()],
            empty_message: "Shard discovery is summarized at stream level for now.".to_owned(),
        },
        ResourceTab {
            key: "consumers".to_owned(),
            label: "Consumers".to_owned(),
            kinds: vec!["consumer".to_owned()],
            empty_message: "No Kinesis consumers are loaded.".to_owned(),
        },
        ResourceTab {
            key: "metrics".to_owned(),
            label: "Metrics".to_owned(),
            kinds: vec!["metric".to_owned()],
            empty_message: "No Kinesis metrics are loaded.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_kinesis::Client {
    let sdk_config = aws_sdk_kinesis::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_kinesis::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_kinesis::Client::from_conf(sdk_config)
}
