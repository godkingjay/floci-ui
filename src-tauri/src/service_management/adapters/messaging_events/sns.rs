use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        actions::{require_name_payload, require_typed_confirmation},
        adapters::messaging_events::{
            local_credentials, managed_inventory, require_payload_text, require_resource_id,
            resource_name_from_arn, unsupported_action,
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
    let topics = client
        .list_topics()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("sns", "list_topics", err))?;
    let mut resources = Vec::new();

    for topic in topics.topics() {
        let Some(topic_arn) = topic.topic_arn() else {
            continue;
        };
        let attributes = client
            .get_topic_attributes()
            .topic_arn(topic_arn)
            .send()
            .await
            .ok()
            .and_then(|output| {
                output.attributes().map(|attributes| {
                    attributes
                        .iter()
                        .map(|(key, value)| (key.to_owned(), value.to_owned()))
                        .collect::<BTreeMap<_, _>>()
                })
            })
            .unwrap_or_default();

        resources.push(ResourceSummary {
            id: format!("topic/{topic_arn}"),
            name: resource_name_from_arn(topic_arn).to_owned(),
            kind: "topic".to_owned(),
            status: "available".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes: attributes.clone(),
        });

        if let Some(policy) = attributes.get("Policy") {
            resources.push(ResourceSummary {
                id: format!("policy/{topic_arn}"),
                name: format!("{} policy", resource_name_from_arn(topic_arn)),
                kind: "policy".to_owned(),
                status: "attached".to_owned(),
                created_at: None,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes: BTreeMap::from([
                    ("topic_arn".to_owned(), topic_arn.to_owned()),
                    ("policy".to_owned(), policy.to_owned()),
                ]),
            });
        }

        if let Some(delivery_policy) = attributes.get("DeliveryPolicy") {
            resources.push(ResourceSummary {
                id: format!("delivery-policy/{topic_arn}"),
                name: format!("{} delivery", resource_name_from_arn(topic_arn)),
                kind: "delivery-policy".to_owned(),
                status: "configured".to_owned(),
                created_at: None,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes: BTreeMap::from([
                    ("topic_arn".to_owned(), topic_arn.to_owned()),
                    ("delivery_policy".to_owned(), delivery_policy.to_owned()),
                ]),
            });
        }
    }

    let subscriptions =
        client.list_subscriptions().send().await.map_err(|err| {
            ServiceManagementError::client_error("sns", "list_subscriptions", err)
        })?;
    for subscription in subscriptions.subscriptions() {
        let Some(subscription_arn) = subscription.subscription_arn() else {
            continue;
        };
        let name = resource_name_from_arn(subscription_arn);
        let mut attributes = BTreeMap::new();
        if let Some(topic_arn) = subscription.topic_arn() {
            attributes.insert("topic_arn".to_owned(), topic_arn.to_owned());
        }
        if let Some(protocol) = subscription.protocol() {
            attributes.insert("protocol".to_owned(), protocol.to_owned());
        }
        if let Some(endpoint) = subscription.endpoint() {
            attributes.insert("endpoint".to_owned(), endpoint.to_owned());
        }

        resources.push(ResourceSummary {
            id: format!("subscription/{subscription_arn}"),
            name: name.to_owned(),
            kind: "subscription".to_owned(),
            status: "subscribed".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    Ok(managed_inventory(
        "sns",
        "SNS",
        tabs(),
        resources,
        Vec::new(),
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_topic" => {
            let topic_name = require_name_payload(request, "topic_name")?;
            let output = client(config)
                .create_topic()
                .name(&topic_name)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("sns", "create_topic", err))?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created SNS topic `{topic_name}`."),
                resource_id: output.topic_arn().map(|arn| format!("topic/{arn}")),
            })
        }
        "delete_topic" => {
            let topic_name = require_typed_confirmation(request)?;
            let topic_arn = require_resource_id(request, "topic/")?;
            client(config)
                .delete_topic()
                .topic_arn(topic_arn)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("sns", "delete_topic", err))?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted SNS topic `{topic_name}`."),
                resource_id: Some(format!("topic/{topic_arn}")),
            })
        }
        "publish_message" => {
            let topic_arn = require_resource_id(request, "topic/")?;
            let message = require_payload_text(request, "message")?;
            let output = client(config)
                .publish()
                .topic_arn(topic_arn)
                .message(&message)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("sns", "publish_message", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!(
                    "Published SNS message `{}`.",
                    output.message_id().unwrap_or("local-message")
                ),
                resource_id: Some(format!("topic/{topic_arn}")),
            })
        }
        "subscribe_endpoint" => {
            let topic_arn = require_resource_id(request, "topic/")?;
            let protocol = require_payload_text(request, "protocol")?;
            let endpoint = require_payload_text(request, "endpoint")?;
            let output = client(config)
                .subscribe()
                .topic_arn(topic_arn)
                .protocol(&protocol)
                .endpoint(&endpoint)
                .return_subscription_arn(true)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("sns", "subscribe_endpoint", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Subscribed `{protocol}` endpoint to SNS topic."),
                resource_id: output
                    .subscription_arn()
                    .map(|arn| format!("subscription/{arn}")),
            })
        }
        "delete_subscription" => {
            let subscription_name = require_typed_confirmation(request)?;
            let subscription_arn = require_resource_id(request, "subscription/")?;
            client(config)
                .unsubscribe()
                .subscription_arn(subscription_arn)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("sns", "delete_subscription", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Unsubscribed SNS endpoint `{subscription_name}`."),
                resource_id: Some(format!("subscription/{subscription_arn}")),
            })
        }
        "refresh_topic_metadata" => {
            let topic_arn = require_resource_id(request, "topic/")?;
            Ok(ActionResult {
                changed: false,
                message: "Topic metadata refresh completed.".to_owned(),
                resource_id: Some(format!("topic/{topic_arn}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "topics".to_owned(),
            label: "Topics".to_owned(),
            kinds: vec!["topic".to_owned()],
            empty_message: "No SNS topics were found in the local emulator.".to_owned(),
        },
        ResourceTab {
            key: "subscriptions".to_owned(),
            label: "Subscriptions".to_owned(),
            kinds: vec!["subscription".to_owned()],
            empty_message: "No SNS subscriptions are loaded.".to_owned(),
        },
        ResourceTab {
            key: "policies".to_owned(),
            label: "Policies".to_owned(),
            kinds: vec!["policy".to_owned()],
            empty_message: "No SNS policies are loaded.".to_owned(),
        },
        ResourceTab {
            key: "delivery".to_owned(),
            label: "Delivery".to_owned(),
            kinds: vec!["delivery-policy".to_owned()],
            empty_message: "No SNS delivery policies are loaded.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_sns::Client {
    let sdk_config = aws_sdk_sns::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_sns::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_sns::Client::from_conf(sdk_config)
}
