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

    let identities = client.list_email_identities().send().await.map_err(|err| {
        ServiceManagementError::client_error("sesv2", "list_email_identities", err)
    })?;
    for identity in identities.email_identities() {
        let Some(name) = identity.identity_name() else {
            continue;
        };
        resources.push(ResourceSummary {
            id: format!("email-identity/{name}"),
            name: name.to_owned(),
            kind: "email-identity".to_owned(),
            status: identity
                .verification_status()
                .map_or_else(|| "listed".to_owned(), ToString::to_string),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes: BTreeMap::new(),
        });
    }

    if let Ok(configuration_sets) = client.list_configuration_sets().send().await {
        for configuration_set in configuration_sets.configuration_sets() {
            resources.push(ResourceSummary {
                id: format!("configuration-set/{configuration_set}"),
                name: configuration_set.to_owned(),
                kind: "configuration-set".to_owned(),
                status: "available".to_owned(),
                created_at: None,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes: BTreeMap::new(),
            });
        }
    }

    if let Ok(templates) = client.list_email_templates().send().await {
        for template in templates.templates_metadata() {
            let Some(name) = template.template_name() else {
                continue;
            };
            resources.push(ResourceSummary {
                id: format!("template/{name}"),
                name: name.to_owned(),
                kind: "template".to_owned(),
                status: "available".to_owned(),
                created_at: format_timestamp(template.created_timestamp()),
                updated_at: None,
                tags: BTreeMap::new(),
                attributes: BTreeMap::new(),
            });
        }
    }

    if let Ok(contact_lists) = client.list_contact_lists().send().await {
        for contact_list in contact_lists.contact_lists() {
            let Some(name) = contact_list.contact_list_name() else {
                continue;
            };
            resources.push(ResourceSummary {
                id: format!("contact-list/{name}"),
                name: name.to_owned(),
                kind: "contact-list".to_owned(),
                status: "available".to_owned(),
                created_at: None,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes: BTreeMap::new(),
            });
        }
    }

    if let Ok(account) = client.get_account().send().await {
        let mut attributes = BTreeMap::new();
        insert_attr(
            &mut attributes,
            "dedicated_ip_auto_warmup_enabled",
            Some(account.dedicated_ip_auto_warmup_enabled()),
        );
        insert_attr(
            &mut attributes,
            "production_access_enabled",
            Some(account.production_access_enabled()),
        );
        insert_attr(
            &mut attributes,
            "sending_enabled",
            Some(account.sending_enabled()),
        );
        insert_attr(
            &mut attributes,
            "enforcement_status",
            account.enforcement_status(),
        );
        if let Some(quota) = account.send_quota() {
            insert_attr(
                &mut attributes,
                "max24_hour_send",
                Some(quota.max24_hour_send()),
            );
            insert_attr(
                &mut attributes,
                "max_send_rate",
                Some(quota.max_send_rate()),
            );
            insert_attr(
                &mut attributes,
                "sent_last24_hours",
                Some(quota.sent_last24_hours()),
            );
        }

        resources.push(ResourceSummary {
            id: "account/default".to_owned(),
            name: "Account settings".to_owned(),
            kind: "account-setting".to_owned(),
            status: if account.sending_enabled() {
                "sending-enabled"
            } else {
                "sending-disabled"
            }
            .to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    Ok(read_only_inventory("sesv2", "SES v2", tabs(), resources))
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "email-identities".to_owned(),
            label: "Identities".to_owned(),
            kinds: vec!["email-identity".to_owned()],
            empty_message: "No SES v2 email identities were found.".to_owned(),
        },
        ResourceTab {
            key: "templates".to_owned(),
            label: "Templates".to_owned(),
            kinds: vec!["template".to_owned()],
            empty_message: "No SES v2 templates are loaded.".to_owned(),
        },
        ResourceTab {
            key: "configuration-sets".to_owned(),
            label: "Configuration Sets".to_owned(),
            kinds: vec!["configuration-set".to_owned()],
            empty_message: "No SES v2 configuration sets are loaded.".to_owned(),
        },
        ResourceTab {
            key: "contact-lists".to_owned(),
            label: "Contact Lists".to_owned(),
            kinds: vec!["contact-list".to_owned()],
            empty_message: "No SES v2 contact lists are loaded.".to_owned(),
        },
        ResourceTab {
            key: "account-settings".to_owned(),
            label: "Account Settings".to_owned(),
            kinds: vec!["account-setting".to_owned()],
            empty_message: "SES v2 account settings are not available.".to_owned(),
        },
        ResourceTab {
            key: "suppression".to_owned(),
            label: "Suppression".to_owned(),
            kinds: vec!["suppression".to_owned()],
            empty_message: "No SES v2 suppression entries are loaded.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_sesv2::Client {
    let sdk_config = aws_sdk_sesv2::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_sesv2::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_sesv2::Client::from_conf(sdk_config)
}
