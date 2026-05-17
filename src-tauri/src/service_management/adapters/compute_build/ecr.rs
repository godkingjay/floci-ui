use std::collections::BTreeMap;

use aws_sdk_ecr::types::ImageIdentifier;

use crate::{
    config::AppConfig,
    service_management::{
        actions::{require_name_payload, require_typed_confirmation},
        adapters::compute_build::{
            format_timestamp, insert_attr, local_credentials, managed_inventory,
            require_resource_id, unsupported_action,
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
    let output =
        client.describe_repositories().send().await.map_err(|err| {
            ServiceManagementError::client_error("ecr", "describe_repositories", err)
        })?;
    let mut resources = Vec::new();

    for repository in output.repositories() {
        let Some(repository_name) = repository.repository_name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();
        insert_attr(
            &mut attributes,
            "repository_arn",
            repository.repository_arn(),
        );
        insert_attr(
            &mut attributes,
            "repository_uri",
            repository.repository_uri(),
        );

        resources.push(ResourceSummary {
            id: format!("repository/{repository_name}"),
            name: repository_name.to_owned(),
            kind: "repository".to_owned(),
            status: "available".to_owned(),
            created_at: format_timestamp(repository.created_at()),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });

        if let Ok(images) = client
            .describe_images()
            .repository_name(repository_name)
            .send()
            .await
        {
            for image in images.image_details() {
                let digest = image.image_digest().unwrap_or("untagged");
                let tag = image
                    .image_tags()
                    .first()
                    .map_or("untagged", String::as_str);
                let mut attributes =
                    BTreeMap::from([("repository_name".to_owned(), repository_name.to_owned())]);
                insert_attr(&mut attributes, "image_digest", image.image_digest());
                insert_attr(
                    &mut attributes,
                    "image_size_bytes",
                    image.image_size_in_bytes(),
                );
                insert_attr(
                    &mut attributes,
                    "scan_status",
                    image.image_scan_status().and_then(|status| status.status()),
                );

                resources.push(ResourceSummary {
                    id: format!("image/{repository_name}/{digest}"),
                    name: tag.to_owned(),
                    kind: "image".to_owned(),
                    status: "available".to_owned(),
                    created_at: format_timestamp(image.image_pushed_at()),
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes: attributes.clone(),
                });

                for tag in image.image_tags() {
                    resources.push(ResourceSummary {
                        id: format!("tag/{repository_name}/{tag}"),
                        name: tag.to_owned(),
                        kind: "tag".to_owned(),
                        status: "available".to_owned(),
                        created_at: format_timestamp(image.image_pushed_at()),
                        updated_at: None,
                        tags: BTreeMap::new(),
                        attributes: attributes.clone(),
                    });
                }
            }
        }
    }

    Ok(managed_inventory(
        "ecr",
        "ECR",
        tabs(),
        resources,
        vec![
            "put_lifecycle_policy".to_owned(),
            "start_image_scan".to_owned(),
        ],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_repository" => {
            let repository_name = require_name_payload(request, "repository_name")?;
            client(config)
                .create_repository()
                .repository_name(&repository_name)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("ecr", "create_repository", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created ECR repository `{repository_name}`."),
                resource_id: Some(format!("repository/{repository_name}")),
            })
        }
        "delete_repository" => {
            let repository_name = require_typed_confirmation(request)?;
            client(config)
                .delete_repository()
                .repository_name(&repository_name)
                .force(true)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("ecr", "delete_repository", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted ECR repository `{repository_name}`."),
                resource_id: Some(format!("repository/{repository_name}")),
            })
        }
        "delete_image" => {
            let image_id = require_resource_id(request, "image/")?;
            let Some((repository_name, image_digest)) = image_id.split_once('/') else {
                return Err(ServiceManagementError::invalid_input(
                    request.service_key.clone(),
                    request.action.clone(),
                    "Image resource id must include repository and digest.",
                ));
            };

            client(config)
                .batch_delete_image()
                .repository_name(repository_name)
                .image_ids(
                    ImageIdentifier::builder()
                        .image_digest(image_digest)
                        .build(),
                )
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("ecr", "batch_delete_image", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted ECR image `{image_digest}`."),
                resource_id: Some(format!("image/{repository_name}/{image_digest}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        tab(
            "repositories",
            "Repositories",
            &["repository"],
            "No ECR repositories were found.",
        ),
        tab("images", "Images", &["image"], "No ECR images are loaded."),
        tab("tags", "Tags", &["tag"], "No ECR image tags are loaded."),
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

fn client(config: &AppConfig) -> aws_sdk_ecr::Client {
    let sdk_config = aws_sdk_ecr::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_ecr::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_ecr::Client::from_conf(sdk_config)
}
