use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        adapters::messaging_events::{
            format_timestamp, insert_attr, local_credentials, read_only_inventory,
        },
        errors::ServiceManagementError,
        models::{ResourceSummary, ResourceTab, ServiceInventory},
    },
};

pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let client = client(config);
    let mut resources = Vec::new();

    let identities = client
        .list_identities()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("ses", "list_identities", err))?;
    for identity in identities.identities() {
        resources.push(ResourceSummary {
            id: format!("identity/{identity}"),
            name: identity.to_owned(),
            kind: "identity".to_owned(),
            status: "listed".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes: BTreeMap::new(),
        });
    }

    if let Ok(templates) = client.list_templates().send().await {
        for template in templates.templates_metadata() {
            let Some(name) = template.name() else {
                continue;
            };
            resources.push(ResourceSummary {
                id: format!("template/{name}"),
                name: name.to_owned(),
                kind: "template".to_owned(),
                status: "available".to_owned(),
                created_at: None,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes: BTreeMap::new(),
            });
        }
    }

    if let Ok(configuration_sets) = client.list_configuration_sets().send().await {
        for configuration_set in configuration_sets.configuration_sets() {
            resources.push(ResourceSummary {
                id: format!("configuration-set/{}", configuration_set.name()),
                name: configuration_set.name().to_owned(),
                kind: "configuration-set".to_owned(),
                status: "available".to_owned(),
                created_at: None,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes: BTreeMap::new(),
            });
        }
    }

    if let Ok(statistics) = client.get_send_statistics().send().await {
        for (index, data_point) in statistics.send_data_points().iter().enumerate() {
            let mut attributes = BTreeMap::new();
            insert_attr(
                &mut attributes,
                "delivery_attempts",
                Some(data_point.delivery_attempts()),
            );
            insert_attr(&mut attributes, "bounces", Some(data_point.bounces()));
            insert_attr(&mut attributes, "complaints", Some(data_point.complaints()));
            insert_attr(&mut attributes, "rejects", Some(data_point.rejects()));
            let timestamp = format_timestamp(data_point.timestamp());

            resources.push(ResourceSummary {
                id: format!("send-statistic/{index}"),
                name: timestamp
                    .clone()
                    .unwrap_or_else(|| format!("send-statistic-{index}")),
                kind: "send-statistic".to_owned(),
                status: "available".to_owned(),
                created_at: timestamp,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes,
            });
        }
    }

    Ok(read_only_inventory("ses", "SES", tabs(), resources))
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "identities".to_owned(),
            label: "Identities".to_owned(),
            kinds: vec!["identity".to_owned()],
            empty_message: "No SES identities were found.".to_owned(),
        },
        ResourceTab {
            key: "templates".to_owned(),
            label: "Templates".to_owned(),
            kinds: vec!["template".to_owned()],
            empty_message: "No SES templates are loaded.".to_owned(),
        },
        ResourceTab {
            key: "configuration-sets".to_owned(),
            label: "Configuration Sets".to_owned(),
            kinds: vec!["configuration-set".to_owned()],
            empty_message: "No SES configuration sets are loaded.".to_owned(),
        },
        ResourceTab {
            key: "suppression".to_owned(),
            label: "Suppression".to_owned(),
            kinds: vec!["suppression".to_owned()],
            empty_message: "No SES suppression entries are loaded.".to_owned(),
        },
        ResourceTab {
            key: "send-statistics".to_owned(),
            label: "Send Statistics".to_owned(),
            kinds: vec!["send-statistic".to_owned()],
            empty_message: "No SES send statistics are loaded.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_ses::Client {
    let sdk_config = aws_sdk_ses::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_ses::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_ses::Client::from_conf(sdk_config)
}
