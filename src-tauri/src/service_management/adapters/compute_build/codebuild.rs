use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        adapters::compute_build::{
            format_timestamp, insert_attr, local_credentials, managed_inventory,
            optional_payload_text, require_resource_id, unsupported_action,
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
    let project_names = client
        .list_projects()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("codebuild", "list_projects", err))?
        .projects()
        .to_vec();
    let mut resources = Vec::new();

    if !project_names.is_empty() {
        let projects = client
            .batch_get_projects()
            .set_names(Some(project_names))
            .send()
            .await
            .map_err(|err| {
                ServiceManagementError::client_error("codebuild", "batch_get_projects", err)
            })?;

        for project in projects.projects() {
            let Some(project_name) = project.name() else {
                continue;
            };
            let mut attributes = BTreeMap::new();
            insert_attr(&mut attributes, "arn", project.arn());
            insert_attr(&mut attributes, "description", project.description());
            if let Some(source) = project.source() {
                insert_attr(&mut attributes, "source_type", Some(source.r#type()));
                insert_attr(&mut attributes, "source_location", source.location());
            }
            if let Some(environment) = project.environment() {
                insert_attr(
                    &mut attributes,
                    "environment_type",
                    Some(environment.r#type()),
                );
                insert_attr(
                    &mut attributes,
                    "compute_type",
                    Some(environment.compute_type()),
                );
                insert_attr(&mut attributes, "image", Some(environment.image()));
            }

            resources.push(ResourceSummary {
                id: format!("project/{project_name}"),
                name: project_name.to_owned(),
                kind: "project".to_owned(),
                status: "available".to_owned(),
                created_at: format_timestamp(project.created()),
                updated_at: format_timestamp(project.last_modified()),
                tags: BTreeMap::new(),
                attributes,
            });
        }
    }

    if let Ok(builds) = client.list_builds().send().await {
        let build_ids = builds.ids().to_vec();
        if !build_ids.is_empty() {
            if let Ok(batch) = client
                .batch_get_builds()
                .set_ids(Some(build_ids))
                .send()
                .await
            {
                for build in batch.builds() {
                    let Some(build_id) = build.id() else {
                        continue;
                    };
                    let mut attributes = BTreeMap::new();
                    insert_attr(&mut attributes, "arn", build.arn());
                    insert_attr(&mut attributes, "project_name", build.project_name());
                    insert_attr(&mut attributes, "source_version", build.source_version());

                    resources.push(ResourceSummary {
                        id: format!("build/{build_id}"),
                        name: build
                            .build_number()
                            .map_or_else(|| build_id.to_owned(), |number| format!("#{number}")),
                        kind: "build".to_owned(),
                        status: build
                            .build_status()
                            .map_or_else(|| "unknown".to_owned(), ToString::to_string),
                        created_at: format_timestamp(build.start_time()),
                        updated_at: format_timestamp(build.end_time()),
                        tags: BTreeMap::new(),
                        attributes,
                    });
                }
            }
        }
    }

    if let Ok(report_groups) = client.list_report_groups().send().await {
        for report_group_arn in report_groups.report_groups() {
            resources.push(ResourceSummary {
                id: format!("report/{report_group_arn}"),
                name: report_group_arn
                    .rsplit('/')
                    .next()
                    .unwrap_or(report_group_arn)
                    .to_owned(),
                kind: "report".to_owned(),
                status: "available".to_owned(),
                created_at: None,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes: BTreeMap::from([(
                    "report_group_arn".to_owned(),
                    report_group_arn.to_owned(),
                )]),
            });
        }
    }

    Ok(managed_inventory(
        "codebuild",
        "CodeBuild",
        tabs(),
        resources,
        vec!["delete_project".to_owned(), "retry_build".to_owned()],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "start_build" => {
            let project_name = require_resource_id(request, "project/")?;
            let mut builder = client(config).start_build().project_name(project_name);
            if let Some(source_version) = optional_payload_text(request, "source_version") {
                builder = builder.source_version(source_version);
            }
            let output = builder.send().await.map_err(|err| {
                ServiceManagementError::client_error("codebuild", "start_build", err)
            })?;
            let build_id = output
                .build_value()
                .and_then(|build| build.id())
                .unwrap_or("local-build");

            Ok(ActionResult {
                changed: true,
                message: format!("Started CodeBuild build `{build_id}`."),
                resource_id: Some(format!("project/{project_name}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        tab(
            "projects",
            "Projects",
            &["project"],
            "No CodeBuild projects were found.",
        ),
        tab(
            "builds",
            "Builds",
            &["build"],
            "No CodeBuild builds are loaded.",
        ),
        tab(
            "reports",
            "Reports",
            &["report"],
            "No CodeBuild reports are loaded.",
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

fn client(config: &AppConfig) -> aws_sdk_codebuild::Client {
    let sdk_config = aws_sdk_codebuild::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_codebuild::config::Region::new(
            config.region.clone(),
        ))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_codebuild::Client::from_conf(sdk_config)
}
