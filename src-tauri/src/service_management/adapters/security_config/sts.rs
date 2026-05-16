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
    let output = client(config)
        .get_caller_identity()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("sts", "get_caller_identity", err))?;

    let account = output.account().unwrap_or("000000000000");
    let arn = output
        .arn()
        .unwrap_or("arn:aws:sts::000000000000:assumed-role/local/floci");
    let user_id = output.user_id().unwrap_or("local");

    let resources = vec![
        ResourceSummary {
            id: "caller-identity/current".to_owned(),
            name: account.to_owned(),
            kind: "caller-identity".to_owned(),
            status: "resolved".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes: BTreeMap::from([
                ("account".to_owned(), account.to_owned()),
                ("arn".to_owned(), arn.to_owned()),
                ("user_id".to_owned(), user_id.to_owned()),
                ("region".to_owned(), config.region.clone()),
                ("credentials".to_owned(), "redacted".to_owned()),
            ]),
        },
        ResourceSummary {
            id: "session/local".to_owned(),
            name: "Local session context".to_owned(),
            kind: "session".to_owned(),
            status: "local".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes: BTreeMap::from([
                ("endpoint".to_owned(), config.endpoint_url.to_string()),
                ("region".to_owned(), config.region.clone()),
                ("access_key_id".to_owned(), "redacted".to_owned()),
                ("secret_access_key".to_owned(), "redacted".to_owned()),
            ]),
        },
    ];

    Ok(read_only_inventory("sts", "STS", tabs(), resources))
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "caller-identity".to_owned(),
            label: "Caller Identity".to_owned(),
            kinds: vec!["caller-identity".to_owned()],
            empty_message: "No STS caller identity is available.".to_owned(),
        },
        ResourceTab {
            key: "sessions".to_owned(),
            label: "Session Context".to_owned(),
            kinds: vec!["session".to_owned()],
            empty_message: "No local session context is available.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_sts::Client {
    let sdk_config = aws_sdk_sts::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_sts::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_sts::Client::from_conf(sdk_config)
}
