use std::collections::BTreeMap;

use aws_credential_types::Credentials;

use crate::{
    config::AppConfig,
    service_management::{
        actions::{require_name_payload, require_typed_confirmation},
        errors::ServiceManagementError,
        models::{
            ActionResult, ResourceSummary, ResourceTab, ServiceActionRequest, ServiceInventory,
            ServiceSupportLevel,
        },
    },
};

pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let output = client(config)
        .list_buckets()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("s3", "list_buckets", err))?;

    let resources = output
        .buckets()
        .iter()
        .filter_map(|bucket| {
            let name = bucket.name()?.to_owned();
            Some(ResourceSummary {
                id: format!("bucket/{name}"),
                name,
                kind: "bucket".to_owned(),
                status: "available".to_owned(),
                created_at: bucket.creation_date().map(|date| format!("{date:?}")),
                updated_at: None,
                tags: BTreeMap::new(),
                attributes: BTreeMap::from([(
                    "preview".to_owned(),
                    "Object body preview is metadata-only by default.".to_owned(),
                )]),
            })
        })
        .collect();

    Ok(ServiceInventory {
        service_key: "s3".to_owned(),
        service_label: "S3".to_owned(),
        support_level: ServiceSupportLevel::Managed,
        refreshed_at: crate::service_management::actions::now_rfc3339(),
        tabs: tabs(),
        resources,
        unsupported_operations: vec!["empty_bucket".to_owned(), "object_body_preview".to_owned()],
    })
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_bucket" => {
            let bucket_name = require_name_payload(request, "bucket_name")?;
            client(config)
                .create_bucket()
                .bucket(&bucket_name)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("s3", "create_bucket", err))?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created S3 bucket `{bucket_name}`."),
                resource_id: Some(format!("bucket/{bucket_name}")),
            })
        }
        "delete_bucket" => {
            let bucket_name = require_typed_confirmation(request)?;
            client(config)
                .delete_bucket()
                .bucket(&bucket_name)
                .send()
                .await
                .map_err(|err| ServiceManagementError::client_error("s3", "delete_bucket", err))?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted S3 bucket `{bucket_name}`."),
                resource_id: Some(format!("bucket/{bucket_name}")),
            })
        }
        "refresh_bucket_metadata" => Ok(ActionResult {
            changed: false,
            message: "Bucket metadata refresh completed.".to_owned(),
            resource_id: request.resource_id.clone(),
        }),
        "empty_bucket" => Err(ServiceManagementError::unsupported_operation(
            "s3",
            "empty_bucket",
            "Empty bucket is intentionally blocked until object listing and typed object deletion are implemented.",
        )),
        action => Err(ServiceManagementError::unsupported_operation(
            "s3",
            action,
            format!("S3 action `{action}` is not supported."),
        )),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "buckets".to_owned(),
            label: "Buckets".to_owned(),
            kinds: vec!["bucket".to_owned()],
            empty_message: "No S3 buckets were found in the local emulator.".to_owned(),
        },
        ResourceTab {
            key: "object-prefixes".to_owned(),
            label: "Object Prefixes".to_owned(),
            kinds: vec!["object-prefix".to_owned()],
            empty_message: "Object prefix discovery is metadata-only for now.".to_owned(),
        },
        ResourceTab {
            key: "policies".to_owned(),
            label: "Policies".to_owned(),
            kinds: vec!["bucket-policy".to_owned()],
            empty_message: "No bucket policies are loaded.".to_owned(),
        },
        ResourceTab {
            key: "lifecycle".to_owned(),
            label: "Lifecycle".to_owned(),
            kinds: vec!["lifecycle-rule".to_owned()],
            empty_message: "No lifecycle rules are loaded.".to_owned(),
        },
        ResourceTab {
            key: "tags".to_owned(),
            label: "Tags".to_owned(),
            kinds: vec!["tag-set".to_owned()],
            empty_message: "No S3 tag sets are loaded.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_s3::Client {
    let sdk_config = aws_sdk_s3::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_s3::config::Region::new(config.region.clone()))
        .credentials_provider(Credentials::new(
            config.access_key_id.clone(),
            config.secret_access_key.clone(),
            None,
            None,
            "floci-ui",
        ))
        .force_path_style(true)
        .build();

    aws_sdk_s3::Client::from_conf(sdk_config)
}
