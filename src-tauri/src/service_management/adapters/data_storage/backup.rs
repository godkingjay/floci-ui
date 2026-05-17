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
    let mut vault_names = Vec::new();

    let vaults =
        client.list_backup_vaults().send().await.map_err(|err| {
            ServiceManagementError::client_error("backup", "list_backup_vaults", err)
        })?;
    for vault in vaults.backup_vault_list() {
        let Some(name) = vault.backup_vault_name() else {
            continue;
        };
        vault_names.push(name.to_owned());
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "vault_arn", vault.backup_vault_arn());
        insert_attr(
            &mut attributes,
            "encryption_key",
            vault.encryption_key_arn(),
        );
        insert_attr(
            &mut attributes,
            "recovery_points",
            Some(vault.number_of_recovery_points().to_string()),
        );
        insert_attr(
            &mut attributes,
            "locked",
            vault.locked().map(|value| value.to_string()),
        );

        resources.push(ResourceSummary {
            id: format!("backup-vault/{name}"),
            name: name.to_owned(),
            kind: "backup-vault".to_owned(),
            status: "available".to_owned(),
            created_at: format_timestamp(vault.creation_date()),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }

    let plans =
        client.list_backup_plans().send().await.map_err(|err| {
            ServiceManagementError::client_error("backup", "list_backup_plans", err)
        })?;
    for plan in plans.backup_plans_list() {
        let Some(plan_id) = plan.backup_plan_id() else {
            continue;
        };
        let name = plan.backup_plan_name().unwrap_or(plan_id);
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "version", plan.version_id());
        insert_attr(
            &mut attributes,
            "creator_request_id",
            plan.creator_request_id(),
        );

        resources.push(ResourceSummary {
            id: format!("backup-plan/{plan_id}"),
            name: name.to_owned(),
            kind: "backup-plan".to_owned(),
            status: "available".to_owned(),
            created_at: format_timestamp(plan.creation_date()),
            updated_at: format_timestamp(plan.last_execution_date()),
            tags: BTreeMap::new(),
            attributes,
        });
    }

    let jobs =
        client.list_backup_jobs().send().await.map_err(|err| {
            ServiceManagementError::client_error("backup", "list_backup_jobs", err)
        })?;
    for job in jobs.backup_jobs() {
        let Some(job_id) = job.backup_job_id() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "vault", job.backup_vault_name());
        insert_attr(&mut attributes, "resource_arn", job.resource_arn());
        insert_attr(&mut attributes, "status_message", job.status_message());

        resources.push(ResourceSummary {
            id: format!("backup-job/{job_id}"),
            name: job_id.to_owned(),
            kind: "backup-job".to_owned(),
            status: job
                .state()
                .map_or_else(|| "unknown".to_owned(), ToString::to_string),
            created_at: format_timestamp(job.creation_date()),
            updated_at: format_timestamp(job.completion_date()),
            tags: BTreeMap::new(),
            attributes,
        });
    }

    for vault_name in vault_names {
        let recovery_points = client
            .list_recovery_points_by_backup_vault()
            .backup_vault_name(&vault_name)
            .send()
            .await
            .map_err(|err| {
                ServiceManagementError::client_error(
                    "backup",
                    "list_recovery_points_by_backup_vault",
                    err,
                )
            })?;

        for point in recovery_points.recovery_points() {
            let Some(point_arn) = point.recovery_point_arn() else {
                continue;
            };
            let mut attributes = BTreeMap::new();
            insert_attr(&mut attributes, "vault", point.backup_vault_name());
            insert_attr(&mut attributes, "resource_arn", point.resource_arn());
            insert_attr(&mut attributes, "resource_type", point.resource_type());
            insert_attr(&mut attributes, "status_message", point.status_message());

            resources.push(ResourceSummary {
                id: format!("recovery-point/{point_arn}"),
                name: point_arn.to_owned(),
                kind: "recovery-point".to_owned(),
                status: point
                    .status()
                    .map_or_else(|| "unknown".to_owned(), ToString::to_string),
                created_at: format_timestamp(point.creation_date()),
                updated_at: format_timestamp(point.completion_date()),
                tags: BTreeMap::new(),
                attributes,
            });
        }
    }

    Ok(read_only_inventory(
        "backup",
        "AWS Backup",
        tabs(),
        resources,
    ))
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "vaults".to_owned(),
            label: "Vaults".to_owned(),
            kinds: vec!["backup-vault".to_owned()],
            empty_message: "No AWS Backup vaults were found.".to_owned(),
        },
        ResourceTab {
            key: "plans".to_owned(),
            label: "Plans".to_owned(),
            kinds: vec!["backup-plan".to_owned()],
            empty_message: "No AWS Backup plans were found.".to_owned(),
        },
        ResourceTab {
            key: "jobs".to_owned(),
            label: "Jobs".to_owned(),
            kinds: vec!["backup-job".to_owned()],
            empty_message: "No AWS Backup jobs were found.".to_owned(),
        },
        ResourceTab {
            key: "recovery-points".to_owned(),
            label: "Recovery Points".to_owned(),
            kinds: vec!["recovery-point".to_owned()],
            empty_message: "No AWS Backup recovery points were found.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_backup::Client {
    let sdk_config = aws_sdk_backup::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_backup::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_backup::Client::from_conf(sdk_config)
}

fn insert_attr(attributes: &mut BTreeMap<String, String>, key: &str, value: Option<impl ToString>) {
    if let Some(value) = value {
        attributes.insert(key.to_owned(), value.to_string());
    }
}
