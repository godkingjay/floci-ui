use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        adapters::security_config::{local_credentials, read_only_inventory},
        errors::ServiceManagementError,
        models::{ResourceSummary, ResourceTab, ServiceInventory},
    },
};

pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let _client = client(config);
    let resources = vec![
        ResourceSummary {
            id: "session-preview/local".to_owned(),
            name: "Local session preview".to_owned(),
            kind: "session-preview".to_owned(),
            status: "metadata-only".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes: BTreeMap::from([
                ("endpoint".to_owned(), config.endpoint_url.to_string()),
                ("region".to_owned(), config.region.clone()),
                (
                    "token_storage".to_owned(),
                    "configuration tokens are not persisted in the UI".to_owned(),
                ),
            ]),
        },
        ResourceSummary {
            id: "configuration-preview/local".to_owned(),
            name: "Configuration payload preview".to_owned(),
            kind: "configuration-preview".to_owned(),
            status: "redacted".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes: BTreeMap::from([
                (
                    "payload".to_owned(),
                    "not fetched until an explicit AppConfigData session is supported".to_owned(),
                ),
                ("client".to_owned(), "aws-sdk-appconfigdata".to_owned()),
            ]),
        },
    ];

    Ok(read_only_inventory(
        "appconfigdata",
        "AppConfig Data",
        tabs(),
        resources,
    ))
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        tab(
            "session-preview",
            "Session Preview",
            &["session-preview"],
            "No AppConfigData session preview is loaded.",
        ),
        tab(
            "configuration-preview",
            "Configuration Preview",
            &["configuration-preview"],
            "No AppConfigData configuration preview is loaded.",
        ),
    ]
}

fn tab(key: &str, label: &str, kinds: &[&str], empty_message: &str) -> ResourceTab {
    ResourceTab {
        key: key.to_owned(),
        label: label.to_owned(),
        kinds: kinds.iter().map(|kind| (*kind).to_owned()).collect(),
        empty_message: empty_message.to_owned(),
    }
}

fn client(config: &AppConfig) -> aws_sdk_appconfigdata::Client {
    let sdk_config = aws_sdk_appconfigdata::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_appconfigdata::config::Region::new(
            config.region.clone(),
        ))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_appconfigdata::Client::from_conf(sdk_config)
}
