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

    let groups = client.list_schedule_groups().send().await.map_err(|err| {
        ServiceManagementError::client_error("scheduler", "list_schedule_groups", err)
    })?;
    for group in groups.schedule_groups() {
        let Some(name) = group.name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "arn", group.arn());

        resources.push(ResourceSummary {
            id: format!("schedule-group/{name}"),
            name: name.to_owned(),
            kind: "schedule-group".to_owned(),
            status: group
                .state()
                .map_or_else(|| "unknown".to_owned(), ToString::to_string),
            created_at: format_timestamp(group.creation_date()),
            updated_at: format_timestamp(group.last_modification_date()),
            tags: BTreeMap::new(),
            attributes,
        });
    }

    let schedules =
        client.list_schedules().send().await.map_err(|err| {
            ServiceManagementError::client_error("scheduler", "list_schedules", err)
        })?;
    for schedule in schedules.schedules() {
        let Some(name) = schedule.name() else {
            continue;
        };
        let group = schedule.group_name().unwrap_or("default");
        let mut attributes = BTreeMap::from([("group".to_owned(), group.to_owned())]);
        insert_attr(&mut attributes, "arn", schedule.arn());
        if let Some(target) = schedule.target() {
            insert_attr(&mut attributes, "target_arn", Some(target.arn()));
        }

        resources.push(ResourceSummary {
            id: format!("schedule/{group}/{name}"),
            name: name.to_owned(),
            kind: "schedule".to_owned(),
            status: schedule
                .state()
                .map_or_else(|| "unknown".to_owned(), ToString::to_string),
            created_at: format_timestamp(schedule.creation_date()),
            updated_at: format_timestamp(schedule.last_modification_date()),
            tags: BTreeMap::new(),
            attributes,
        });

        if let Some(target) = schedule.target() {
            resources.push(ResourceSummary {
                id: format!("target/{group}/{name}"),
                name: target.arn().to_owned(),
                kind: "target".to_owned(),
                status: "attached".to_owned(),
                created_at: None,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes: BTreeMap::from([
                    ("schedule_group".to_owned(), group.to_owned()),
                    ("schedule_name".to_owned(), name.to_owned()),
                    ("target_arn".to_owned(), target.arn().to_owned()),
                ]),
            });
        }
    }

    Ok(read_only_inventory(
        "scheduler",
        "EventBridge Scheduler",
        tabs(),
        resources,
    ))
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "schedules".to_owned(),
            label: "Schedules".to_owned(),
            kinds: vec!["schedule".to_owned()],
            empty_message: "No EventBridge Scheduler schedules were found.".to_owned(),
        },
        ResourceTab {
            key: "schedule-groups".to_owned(),
            label: "Groups".to_owned(),
            kinds: vec!["schedule-group".to_owned()],
            empty_message: "No EventBridge Scheduler groups are loaded.".to_owned(),
        },
        ResourceTab {
            key: "targets".to_owned(),
            label: "Targets".to_owned(),
            kinds: vec!["target".to_owned()],
            empty_message: "Schedule targets are summarized on schedule resources.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_scheduler::Client {
    let sdk_config = aws_sdk_scheduler::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_scheduler::config::Region::new(
            config.region.clone(),
        ))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_scheduler::Client::from_conf(sdk_config)
}
