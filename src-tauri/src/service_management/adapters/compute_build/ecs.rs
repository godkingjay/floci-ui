use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        adapters::compute_build::{
            insert_attr, local_credentials, managed_inventory, optional_payload_i32,
            optional_payload_text, require_payload_text, require_resource_id, unsupported_action,
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
    let cluster_arns = client
        .list_clusters()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("ecs", "list_clusters", err))?
        .cluster_arns()
        .to_vec();
    let mut resources = Vec::new();

    if !cluster_arns.is_empty() {
        let clusters = client
            .describe_clusters()
            .set_clusters(Some(cluster_arns.clone()))
            .send()
            .await
            .map_err(|err| ServiceManagementError::client_error("ecs", "describe_clusters", err))?;
        for cluster in clusters.clusters() {
            let Some(cluster_arn) = cluster.cluster_arn() else {
                continue;
            };
            let cluster_name = cluster
                .cluster_name()
                .unwrap_or_else(|| name_from_arn(cluster_arn));
            let mut attributes = BTreeMap::new();
            insert_attr(&mut attributes, "cluster_arn", Some(cluster_arn));
            insert_attr(
                &mut attributes,
                "running_tasks",
                Some(cluster.running_tasks_count()),
            );
            insert_attr(
                &mut attributes,
                "pending_tasks",
                Some(cluster.pending_tasks_count()),
            );
            insert_attr(
                &mut attributes,
                "active_services",
                Some(cluster.active_services_count()),
            );

            resources.push(ResourceSummary {
                id: format!("cluster/{cluster_arn}"),
                name: cluster_name.to_owned(),
                kind: "cluster".to_owned(),
                status: cluster.status().unwrap_or("active").to_owned(),
                created_at: None,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes,
            });

            collect_services(&client, cluster_arn, &mut resources).await;
            collect_tasks(&client, cluster_arn, &mut resources).await;
            collect_container_instances(&client, cluster_arn, &mut resources).await;
        }
    }

    collect_task_definitions(&client, &mut resources).await;

    Ok(managed_inventory(
        "ecs",
        "ECS",
        tabs(),
        resources,
        vec!["create_service".to_owned(), "delete_cluster".to_owned()],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "run_task" => {
            let cluster_arn = require_resource_id(request, "cluster/")?;
            let task_definition = require_payload_text(request, "task_definition")?;
            let output = client(config)
                .run_task()
                .cluster(cluster_arn)
                .task_definition(&task_definition)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("ecs", "run_task", err))?;

            Ok(ActionResult {
                changed: true,
                message: format!("Started {} ECS task(s).", output.tasks().len()),
                resource_id: Some(format!("cluster/{cluster_arn}")),
            })
        }
        "stop_task" => {
            let task_arn = require_resource_id(request, "task/")?;
            let cluster_arn = optional_payload_text(request, "cluster_arn")
                .or_else(|| request.resource_name.clone())
                .unwrap_or_default();
            client(config)
                .stop_task()
                .cluster(cluster_arn)
                .task(task_arn)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("ecs", "stop_task", err))?;

            Ok(ActionResult {
                changed: true,
                message: format!("Stopped ECS task `{task_arn}`."),
                resource_id: Some(format!("task/{task_arn}")),
            })
        }
        "update_service_desired_count" => {
            let service_arn = require_resource_id(request, "service/")?;
            let desired_count =
                optional_payload_i32(request, "desired_count").ok_or_else(|| {
                    ServiceManagementError::invalid_input(
                        request.service_key.clone(),
                        request.action.clone(),
                        "`desired_count` is required.",
                    )
                })?;
            let cluster_arn = require_payload_text(request, "cluster_arn")?;
            client(config)
                .update_service()
                .cluster(cluster_arn)
                .service(service_arn)
                .desired_count(desired_count)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("ecs", "update_service", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!(
                    "Updated ECS service `{}` desired count to {desired_count}.",
                    name_from_arn(service_arn)
                ),
                resource_id: Some(format!("service/{service_arn}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        tab(
            "clusters",
            "Clusters",
            &["cluster"],
            "No ECS clusters were found.",
        ),
        tab(
            "services",
            "Services",
            &["service"],
            "No ECS services are loaded.",
        ),
        tab("tasks", "Tasks", &["task"], "No ECS tasks are loaded."),
        tab(
            "task-definitions",
            "Task Definitions",
            &["task-definition"],
            "No ECS task definitions are loaded.",
        ),
        tab(
            "container-instances",
            "Container Instances",
            &["container-instance"],
            "No ECS container instances are loaded.",
        ),
    ]
}

async fn collect_services(
    client: &aws_sdk_ecs::Client,
    cluster_arn: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(service_arns) = client.list_services().cluster(cluster_arn).send().await else {
        return;
    };
    if service_arns.service_arns().is_empty() {
        return;
    }
    let Ok(services) = client
        .describe_services()
        .cluster(cluster_arn)
        .set_services(Some(service_arns.service_arns().to_vec()))
        .send()
        .await
    else {
        return;
    };

    for service in services.services() {
        let Some(service_arn) = service.service_arn() else {
            continue;
        };
        let mut attributes = BTreeMap::from([("cluster_arn".to_owned(), cluster_arn.to_owned())]);
        insert_attr(
            &mut attributes,
            "desired_count",
            Some(service.desired_count()),
        );
        insert_attr(
            &mut attributes,
            "running_count",
            Some(service.running_count()),
        );
        insert_attr(
            &mut attributes,
            "pending_count",
            Some(service.pending_count()),
        );
        insert_attr(
            &mut attributes,
            "task_definition",
            service.task_definition(),
        );

        resources.push(ResourceSummary {
            id: format!("service/{service_arn}"),
            name: service
                .service_name()
                .unwrap_or_else(|| name_from_arn(service_arn))
                .to_owned(),
            kind: "service".to_owned(),
            status: service.status().unwrap_or("active").to_owned(),
            created_at: super::format_timestamp(service.created_at()),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

async fn collect_tasks(
    client: &aws_sdk_ecs::Client,
    cluster_arn: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(task_arns) = client.list_tasks().cluster(cluster_arn).send().await else {
        return;
    };
    if task_arns.task_arns().is_empty() {
        return;
    }
    let Ok(tasks) = client
        .describe_tasks()
        .cluster(cluster_arn)
        .set_tasks(Some(task_arns.task_arns().to_vec()))
        .send()
        .await
    else {
        return;
    };

    for task in tasks.tasks() {
        let Some(task_arn) = task.task_arn() else {
            continue;
        };
        let mut attributes = BTreeMap::from([("cluster_arn".to_owned(), cluster_arn.to_owned())]);
        insert_attr(
            &mut attributes,
            "task_definition",
            task.task_definition_arn(),
        );
        insert_attr(&mut attributes, "desired_status", task.desired_status());
        insert_attr(&mut attributes, "launch_type", task.launch_type());

        resources.push(ResourceSummary {
            id: format!("task/{task_arn}"),
            name: name_from_arn(task_arn).to_owned(),
            kind: "task".to_owned(),
            status: task.last_status().unwrap_or("unknown").to_owned(),
            created_at: super::format_timestamp(task.created_at()),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

async fn collect_container_instances(
    client: &aws_sdk_ecs::Client,
    cluster_arn: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(instance_arns) = client
        .list_container_instances()
        .cluster(cluster_arn)
        .send()
        .await
    else {
        return;
    };
    if instance_arns.container_instance_arns().is_empty() {
        return;
    }
    let Ok(instances) = client
        .describe_container_instances()
        .cluster(cluster_arn)
        .set_container_instances(Some(instance_arns.container_instance_arns().to_vec()))
        .send()
        .await
    else {
        return;
    };

    for instance in instances.container_instances() {
        let Some(instance_arn) = instance.container_instance_arn() else {
            continue;
        };
        let mut attributes = BTreeMap::from([("cluster_arn".to_owned(), cluster_arn.to_owned())]);
        insert_attr(
            &mut attributes,
            "ec2_instance_id",
            instance.ec2_instance_id(),
        );
        insert_attr(
            &mut attributes,
            "running_tasks",
            Some(instance.running_tasks_count()),
        );
        insert_attr(
            &mut attributes,
            "pending_tasks",
            Some(instance.pending_tasks_count()),
        );

        resources.push(ResourceSummary {
            id: format!("container-instance/{instance_arn}"),
            name: name_from_arn(instance_arn).to_owned(),
            kind: "container-instance".to_owned(),
            status: instance.status().unwrap_or("active").to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

async fn collect_task_definitions(
    client: &aws_sdk_ecs::Client,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(task_definitions) = client.list_task_definitions().send().await else {
        return;
    };

    for task_definition in task_definitions.task_definition_arns() {
        resources.push(ResourceSummary {
            id: format!("task-definition/{task_definition}"),
            name: name_from_arn(task_definition).to_owned(),
            kind: "task-definition".to_owned(),
            status: "registered".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes: BTreeMap::from([(
                "task_definition_arn".to_owned(),
                task_definition.to_owned(),
            )]),
        });
    }
}

fn tab(key: &str, label: &str, kinds: &[&str], empty_message: &str) -> ResourceTab {
    ResourceTab {
        key: key.to_owned(),
        label: label.to_owned(),
        kinds: kinds.iter().map(|kind| (*kind).to_owned()).collect(),
        empty_message: empty_message.to_owned(),
    }
}

fn client(config: &AppConfig) -> aws_sdk_ecs::Client {
    let sdk_config = aws_sdk_ecs::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_ecs::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_ecs::Client::from_conf(sdk_config)
}

fn name_from_arn(arn: &str) -> &str {
    arn.rsplit('/').next().unwrap_or(arn)
}
