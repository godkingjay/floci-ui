use std::collections::BTreeMap;

use aws_sdk_sqs::types::QueueAttributeName;

use crate::{
    config::AppConfig,
    service_management::{
        actions::{require_name_payload, require_typed_confirmation},
        adapters::messaging_events::{
            local_credentials, managed_inventory, require_payload_text, require_resource_id,
            unsupported_action,
        },
        errors::ServiceManagementError,
        models::{
            ActionResult, ResourceSummary, ResourceTab, ServiceActionRequest, ServiceInventory,
        },
    },
};

pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let client = client(config);
    let output = client
        .list_queues()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("sqs", "list_queues", err))?;
    let mut resources = Vec::new();

    for queue_url in output.queue_urls() {
        let queue_name = queue_name(queue_url);
        let attributes = client
            .get_queue_attributes()
            .queue_url(queue_url)
            .attribute_names(QueueAttributeName::All)
            .send()
            .await
            .ok()
            .and_then(|output| {
                output.attributes().map(|attributes| {
                    attributes
                        .iter()
                        .map(|(key, value)| (format!("{key:?}"), value.to_owned()))
                        .collect::<BTreeMap<_, _>>()
                })
            })
            .unwrap_or_default();

        resources.push(ResourceSummary {
            id: format!("queue/{queue_url}"),
            name: queue_name.to_owned(),
            kind: "queue".to_owned(),
            status: "available".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes: attributes.clone(),
        });

        for (key, value) in &attributes {
            resources.push(ResourceSummary {
                id: format!("queue-attribute/{queue_name}/{key}"),
                name: key.to_owned(),
                kind: "queue-attribute".to_owned(),
                status: "loaded".to_owned(),
                created_at: None,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes: BTreeMap::from([
                    ("queue_name".to_owned(), queue_name.to_owned()),
                    ("value".to_owned(), value.to_owned()),
                ]),
            });
        }

        if let Some(redrive_policy) = attributes.get("RedrivePolicy") {
            resources.push(ResourceSummary {
                id: format!("dead-letter-queue/{queue_name}"),
                name: format!("{queue_name} DLQ policy"),
                kind: "dead-letter-queue".to_owned(),
                status: "configured".to_owned(),
                created_at: None,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes: BTreeMap::from([
                    ("queue_name".to_owned(), queue_name.to_owned()),
                    ("redrive_policy".to_owned(), redrive_policy.to_owned()),
                ]),
            });
        }
    }

    Ok(managed_inventory(
        "sqs",
        "SQS",
        tabs(),
        resources,
        vec!["message_preview".to_owned()],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_queue" => {
            let queue_name = require_name_payload(request, "queue_name")?;
            client(config)
                .create_queue()
                .queue_name(&queue_name)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("sqs", "create_queue", err))?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created SQS queue `{queue_name}`."),
                resource_id: Some(format!("queue/{queue_name}")),
            })
        }
        "delete_queue" => {
            let queue_name = require_typed_confirmation(request)?;
            let queue_url = resolve_queue_url(config, request, &queue_name).await?;

            client(config)
                .delete_queue()
                .queue_url(&queue_url)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("sqs", "delete_queue", err))?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted SQS queue `{queue_name}`."),
                resource_id: Some(format!("queue/{queue_url}")),
            })
        }
        "send_message" => {
            let queue_url = require_resource_id(request, "queue/")?;
            let message_body = require_payload_text(request, "message_body")?;
            let output = client(config)
                .send_message()
                .queue_url(queue_url)
                .message_body(&message_body)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("sqs", "send_message", err))?;

            Ok(ActionResult {
                changed: true,
                message: format!(
                    "Sent SQS message `{}`.",
                    output.message_id().unwrap_or("local-message")
                ),
                resource_id: Some(format!("queue/{queue_url}")),
            })
        }
        "purge_queue" => {
            let queue_name = require_typed_confirmation(request)?;
            let queue_url = resolve_queue_url(config, request, &queue_name).await?;
            client(config)
                .purge_queue()
                .queue_url(&queue_url)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("sqs", "purge_queue", err))?;

            Ok(ActionResult {
                changed: true,
                message: format!("Purged SQS queue `{queue_name}`."),
                resource_id: Some(format!("queue/{queue_url}")),
            })
        }
        "refresh_queue_attributes" => {
            let queue_url = require_resource_id(request, "queue/")?;
            Ok(ActionResult {
                changed: false,
                message: "Queue attributes refresh completed.".to_owned(),
                resource_id: Some(format!("queue/{queue_url}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "queues".to_owned(),
            label: "Queues".to_owned(),
            kinds: vec!["queue".to_owned()],
            empty_message: "No SQS queues were found in the local emulator.".to_owned(),
        },
        ResourceTab {
            key: "dead-letter-queues".to_owned(),
            label: "Dead Letter Queues".to_owned(),
            kinds: vec!["dead-letter-queue".to_owned()],
            empty_message: "No SQS dead-letter queue policies are loaded.".to_owned(),
        },
        ResourceTab {
            key: "attributes".to_owned(),
            label: "Attributes".to_owned(),
            kinds: vec!["queue-attribute".to_owned()],
            empty_message: "No SQS queue attributes are loaded.".to_owned(),
        },
        ResourceTab {
            key: "messages".to_owned(),
            label: "Messages Preview".to_owned(),
            kinds: vec!["message".to_owned()],
            empty_message: "Message previews are disabled by default.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_sqs::Client {
    let sdk_config = aws_sdk_sqs::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_sqs::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_sqs::Client::from_conf(sdk_config)
}

async fn resolve_queue_url(
    config: &AppConfig,
    request: &ServiceActionRequest,
    queue_name: &str,
) -> Result<String, ServiceManagementError> {
    if let Some(queue_url) = request.resource_id.as_deref().and_then(|value| {
        value
            .strip_prefix("queue/")
            .filter(|queue_url| queue_url.starts_with("http"))
    }) {
        return Ok(queue_url.to_owned());
    }

    client(config)
        .get_queue_url()
        .queue_name(queue_name)
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("sqs", "get_queue_url", err))?
        .queue_url()
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            ServiceManagementError::not_found(
                "sqs",
                "get_queue_url",
                format!("Queue `{queue_name}` was not found."),
            )
        })
}

fn queue_name(queue_url: &str) -> &str {
    queue_url.rsplit('/').next().unwrap_or(queue_url)
}
