use std::collections::BTreeMap;

use aws_sdk_route53::types::{
    Change, ChangeAction, ChangeBatch, ResourceRecord, ResourceRecordSet, RrType,
};
use time::OffsetDateTime;

use crate::{
    config::AppConfig,
    service_management::{
        actions::{require_name_payload, require_typed_confirmation},
        adapters::network_observability::{
            build_error, insert_attr, local_credentials, managed_inventory, optional_payload_i32,
            optional_payload_text, require_payload_text, require_resource_id, unsupported_action,
        },
        errors::ServiceManagementError,
        models::{
            ActionResult, ResourceSummary, ResourceTab, ServiceActionRequest, ServiceInventory,
        },
    },
};

const RECORD_SEPARATOR: &str = "||";

pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let client = client(config);
    let hosted_zones =
        client.list_hosted_zones().send().await.map_err(|err| {
            ServiceManagementError::client_error("route53", "list_hosted_zones", err)
        })?;
    let mut resources = Vec::new();

    for zone in hosted_zones.hosted_zones() {
        let zone_id = zone.id();
        let mut attributes = BTreeMap::new();

        insert_attr(
            &mut attributes,
            "caller_reference",
            Some(zone.caller_reference()),
        );
        insert_attr(
            &mut attributes,
            "resource_record_set_count",
            zone.resource_record_set_count(),
        );
        if let Some(config) = zone.config() {
            insert_attr(&mut attributes, "comment", config.comment());
            insert_attr(&mut attributes, "private_zone", Some(config.private_zone()));
        }

        resources.push(ResourceSummary {
            id: format!("hosted-zone/{zone_id}"),
            name: zone.name().to_owned(),
            kind: "hosted-zone".to_owned(),
            status: zone
                .config()
                .map(|config| {
                    if config.private_zone() {
                        "private".to_owned()
                    } else {
                        "public".to_owned()
                    }
                })
                .unwrap_or_else(|| "available".to_owned()),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });

        load_record_sets(&client, zone_id, &mut resources).await;
    }

    load_health_checks(&client, &mut resources).await;

    Ok(managed_inventory(
        "route53",
        "Route 53",
        tabs(),
        resources,
        vec!["change_history_not_available".to_owned()],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_hosted_zone" => {
            let zone_name = require_name_payload(request, "zone_name")?;
            let caller_reference = format!(
                "floci-ui-{}",
                OffsetDateTime::now_utc().unix_timestamp_nanos()
            );
            let output = client(config)
                .create_hosted_zone()
                .name(&zone_name)
                .caller_reference(caller_reference)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("route53", "create_hosted_zone", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created hosted zone `{zone_name}`."),
                resource_id: output
                    .hosted_zone()
                    .map(|zone| format!("hosted-zone/{}", zone.id())),
            })
        }
        "upsert_record" => {
            let hosted_zone_id = require_resource_id(request, "hosted-zone/")?;
            let record_name = require_payload_text(request, "record_name")?;
            let record_type = RrType::from(
                optional_payload_text(request, "record_type")
                    .unwrap_or_else(|| "A".to_owned())
                    .to_ascii_uppercase()
                    .as_str(),
            );
            let record_value = require_payload_text(request, "record_value")?;
            let ttl = i64::from(optional_payload_i32(request, "ttl").unwrap_or(300));
            let change_batch = single_record_change(
                request,
                ChangeAction::Upsert,
                &record_name,
                record_type,
                ttl,
                &record_value,
            )?;
            let output = client(config)
                .change_resource_record_sets()
                .hosted_zone_id(clean_zone_id(hosted_zone_id))
                .change_batch(change_batch)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error(
                        "route53",
                        "change_resource_record_sets",
                        err,
                    )
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!(
                    "Submitted Route 53 record upsert `{}` ({:?}).",
                    record_name,
                    output.change_info().map(|change| change.status())
                ),
                resource_id: output
                    .change_info()
                    .map(|change| format!("change-batch/{}", change.id())),
            })
        }
        "delete_record" => {
            let record_name = require_typed_confirmation(request)?;
            let (hosted_zone_id, encoded_name, encoded_type, ttl, value) =
                selected_record(request)?;
            let record_type = RrType::from(encoded_type);
            let change_batch = single_record_change(
                request,
                ChangeAction::Delete,
                encoded_name,
                record_type,
                ttl,
                value,
            )?;
            let output = client(config)
                .change_resource_record_sets()
                .hosted_zone_id(clean_zone_id(hosted_zone_id))
                .change_batch(change_batch)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error(
                        "route53",
                        "change_resource_record_sets",
                        err,
                    )
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Submitted Route 53 record delete `{record_name}`."),
                resource_id: output
                    .change_info()
                    .map(|change| format!("change-batch/{}", change.id())),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "hosted-zones".to_owned(),
            label: "Hosted Zones".to_owned(),
            kinds: vec!["hosted-zone".to_owned()],
            empty_message: "No Route 53 hosted zones were found in the local emulator.".to_owned(),
        },
        ResourceTab {
            key: "records".to_owned(),
            label: "Records".to_owned(),
            kinds: vec!["record-set".to_owned()],
            empty_message: "No Route 53 record sets are loaded.".to_owned(),
        },
        ResourceTab {
            key: "health-checks".to_owned(),
            label: "Health Checks".to_owned(),
            kinds: vec!["health-check".to_owned()],
            empty_message: "No Route 53 health checks are loaded.".to_owned(),
        },
        ResourceTab {
            key: "changes".to_owned(),
            label: "Changes".to_owned(),
            kinds: vec!["change-batch".to_owned()],
            empty_message: "Change history is not exposed by this local Route 53 API.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_route53::Client {
    let sdk_config = aws_sdk_route53::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_route53::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_route53::Client::from_conf(sdk_config)
}

async fn load_record_sets(
    client: &aws_sdk_route53::Client,
    hosted_zone_id: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client
        .list_resource_record_sets()
        .hosted_zone_id(clean_zone_id(hosted_zone_id))
        .send()
        .await
    else {
        return;
    };

    for record in output.resource_record_sets() {
        let record_type = record.r#type().as_str();
        let ttl = record.ttl().unwrap_or(0);
        let value = record
            .resource_records()
            .first()
            .map(|record| record.value())
            .unwrap_or_default();
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "hosted_zone_id", Some(hosted_zone_id));
        insert_attr(&mut attributes, "record_type", Some(record_type));
        insert_attr(&mut attributes, "ttl", record.ttl());
        insert_attr(&mut attributes, "set_identifier", record.set_identifier());
        insert_attr(&mut attributes, "health_check_id", record.health_check_id());
        insert_attr(
            &mut attributes,
            "values",
            Some(
                record
                    .resource_records()
                    .iter()
                    .map(|record| record.value())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        );

        resources.push(ResourceSummary {
            id: format!(
                "record-set/{hosted_zone_id}{RECORD_SEPARATOR}{}{RECORD_SEPARATOR}{record_type}{RECORD_SEPARATOR}{ttl}{RECORD_SEPARATOR}{value}",
                record.name()
            ),
            name: format!("{} {record_type}", record.name()),
            kind: "record-set".to_owned(),
            status: record_type.to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

async fn load_health_checks(
    client: &aws_sdk_route53::Client,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client.list_health_checks().send().await else {
        return;
    };

    for health_check in output.health_checks() {
        let mut attributes = BTreeMap::new();

        insert_attr(
            &mut attributes,
            "caller_reference",
            Some(health_check.caller_reference()),
        );

        resources.push(ResourceSummary {
            id: format!("health-check/{}", health_check.id()),
            name: health_check.id().to_owned(),
            kind: "health-check".to_owned(),
            status: "configured".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

fn single_record_change(
    request: &ServiceActionRequest,
    action: ChangeAction,
    record_name: &str,
    record_type: RrType,
    ttl: i64,
    record_value: &str,
) -> Result<ChangeBatch, ServiceManagementError> {
    let record = ResourceRecord::builder()
        .value(record_value)
        .build()
        .map_err(|err| build_error(request, "record_value", err))?;
    let record_set = ResourceRecordSet::builder()
        .name(record_name)
        .r#type(record_type)
        .ttl(ttl)
        .resource_records(record)
        .build()
        .map_err(|err| build_error(request, "record_set", err))?;
    let change = Change::builder()
        .action(action)
        .resource_record_set(record_set)
        .build()
        .map_err(|err| build_error(request, "change", err))?;

    ChangeBatch::builder()
        .changes(change)
        .build()
        .map_err(|err| build_error(request, "change_batch", err))
}

fn selected_record(
    request: &ServiceActionRequest,
) -> Result<(&str, &str, &str, i64, &str), ServiceManagementError> {
    let selected = require_resource_id(request, "record-set/")?;
    let mut parts = selected.splitn(5, RECORD_SEPARATOR);
    let hosted_zone_id = parts.next().unwrap_or_default();
    let record_name = parts.next().unwrap_or_default();
    let record_type = parts.next().unwrap_or_default();
    let ttl = parts
        .next()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(300);
    let value = parts.next().unwrap_or_default();

    if hosted_zone_id.is_empty()
        || record_name.is_empty()
        || record_type.is_empty()
        || value.is_empty()
    {
        return Err(ServiceManagementError::invalid_input(
            request.service_key.clone(),
            request.action.clone(),
            "Select a Route 53 record before running this action.",
        ));
    }

    Ok((hosted_zone_id, record_name, record_type, ttl, value))
}

fn clean_zone_id(hosted_zone_id: &str) -> &str {
    hosted_zone_id
        .strip_prefix("/hostedzone/")
        .or_else(|| hosted_zone_id.strip_prefix("hostedzone/"))
        .unwrap_or(hosted_zone_id)
}
