use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        adapters::compute_build::{
            format_timestamp, insert_attr, local_credentials, managed_inventory,
            optional_payload_text, require_payload_text, require_resource_id, unsupported_action,
        },
        errors::ServiceManagementError,
        models::{
            ActionResult, ResourceSummary, ResourceTab, ServiceActionRequest, ServiceInventory,
        },
    },
};

#[allow(clippy::too_many_lines)]
pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let client = client(config);
    let applications = client
        .list_applications()
        .send()
        .await
        .map_err(|err| {
            ServiceManagementError::client_error("codedeploy", "list_applications", err)
        })?
        .applications()
        .to_vec();
    let mut resources = Vec::new();

    for application_name in applications {
        resources.push(ResourceSummary {
            id: format!("application/{application_name}"),
            name: application_name.clone(),
            kind: "application".to_owned(),
            status: "available".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes: BTreeMap::new(),
        });

        if let Ok(groups) = client
            .list_deployment_groups()
            .application_name(&application_name)
            .send()
            .await
        {
            for group_name in groups.deployment_groups() {
                let mut attributes =
                    BTreeMap::from([("application_name".to_owned(), application_name.clone())]);
                if let Ok(group) = client
                    .get_deployment_group()
                    .application_name(&application_name)
                    .deployment_group_name(group_name)
                    .send()
                    .await
                {
                    if let Some(info) = group.deployment_group_info() {
                        insert_attr(
                            &mut attributes,
                            "deployment_group_id",
                            info.deployment_group_id(),
                        );
                        insert_attr(&mut attributes, "service_role_arn", info.service_role_arn());
                    }
                }

                resources.push(ResourceSummary {
                    id: format!("deployment-group/{application_name}/{group_name}"),
                    name: group_name.to_owned(),
                    kind: "deployment-group".to_owned(),
                    status: "available".to_owned(),
                    created_at: None,
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes,
                });
            }
        }
    }

    if let Ok(deployments) = client.list_deployments().send().await {
        for deployment_id in deployments.deployments() {
            let mut attributes = BTreeMap::new();
            let mut status = "created".to_owned();
            let mut created_at = None;
            if let Ok(deployment) = client
                .get_deployment()
                .deployment_id(deployment_id)
                .send()
                .await
            {
                if let Some(info) = deployment.deployment_info() {
                    insert_attr(&mut attributes, "application_name", info.application_name());
                    insert_attr(
                        &mut attributes,
                        "deployment_group_name",
                        info.deployment_group_name(),
                    );
                    status = info
                        .status()
                        .map_or_else(|| status.clone(), ToString::to_string);
                    created_at = format_timestamp(info.create_time());
                }
            }

            resources.push(ResourceSummary {
                id: format!("deployment/{deployment_id}"),
                name: deployment_id.to_owned(),
                kind: "deployment".to_owned(),
                status,
                created_at,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes,
            });
        }
    }

    Ok(managed_inventory(
        "codedeploy",
        "CodeDeploy",
        tabs(),
        resources,
        vec![
            "delete_application".to_owned(),
            "delete_deployment_group".to_owned(),
        ],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_deployment" => {
            let (application_name, deployment_group_name) =
                require_resource_id(request, "deployment-group/")
                    .ok()
                    .and_then(|value| {
                        value
                            .split_once('/')
                            .map(|(application, group)| (application.to_owned(), group.to_owned()))
                    })
                    .or_else(|| {
                        Some((
                            require_payload_text(request, "application_name").ok()?,
                            optional_payload_text(request, "deployment_group_name")?,
                        ))
                    })
                    .ok_or_else(|| {
                        ServiceManagementError::invalid_input(
                            request.service_key.clone(),
                            request.action.clone(),
                            "`deployment_group_name` is required.",
                        )
                    })?;

            let output = client(config)
                .create_deployment()
                .application_name(&application_name)
                .deployment_group_name(deployment_group_name)
                .description(
                    optional_payload_text(request, "description")
                        .unwrap_or_else(|| "floci-ui local deployment".to_owned()),
                )
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("codedeploy", "create_deployment", err)
                })?;
            let deployment_id = output.deployment_id().unwrap_or("local-deployment");

            Ok(ActionResult {
                changed: true,
                message: format!("Created CodeDeploy deployment `{deployment_id}`."),
                resource_id: Some(format!("deployment/{deployment_id}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        tab(
            "applications",
            "Applications",
            &["application"],
            "No CodeDeploy applications were found.",
        ),
        tab(
            "deployment-groups",
            "Deployment Groups",
            &["deployment-group"],
            "No CodeDeploy deployment groups are loaded.",
        ),
        tab(
            "deployments",
            "Deployments",
            &["deployment"],
            "No CodeDeploy deployments are loaded.",
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

fn client(config: &AppConfig) -> aws_sdk_codedeploy::Client {
    let sdk_config = aws_sdk_codedeploy::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_codedeploy::config::Region::new(
            config.region.clone(),
        ))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_codedeploy::Client::from_conf(sdk_config)
}
