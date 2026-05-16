use std::fmt;

use aws_credential_types::Credentials;

use crate::{
    config::AppConfig,
    service_management::{
        errors::ServiceManagementError,
        models::{
            ActionResult, ResourceDetail, ResourceDetailRequest, ResourceSummary,
            ServiceActionRequest, ServiceInventory, ServiceSupportLevel,
        },
    },
};

pub mod athena;
pub mod backup;
pub mod dynamodb;
pub mod dynamodbstreams;
pub mod elasticache;
pub mod firehose;
pub mod glue;
pub mod opensearch;
pub mod rds;
pub mod s3;

pub const SERVICE_KEYS: &[&str] = &[
    "s3",
    "dynamodb",
    "dynamodbstreams",
    "rds",
    "elasticache",
    "opensearch",
    "glue",
    "athena",
    "firehose",
    "backup",
];

pub async fn list_resources(
    config: &AppConfig,
    service_key: &str,
) -> Result<ServiceInventory, ServiceManagementError> {
    match service_key {
        "s3" => s3::list_resources(config).await,
        "dynamodb" => dynamodb::list_resources(config).await,
        "dynamodbstreams" => dynamodbstreams::list_resources(config).await,
        "rds" => rds::list_resources(config).await,
        "elasticache" => elasticache::list_resources(config).await,
        "opensearch" => opensearch::list_resources(config).await,
        "glue" => glue::list_resources(config).await,
        "athena" => athena::list_resources(config).await,
        "firehose" => firehose::list_resources(config).await,
        "backup" => backup::list_resources(config).await,
        _ => Err(ServiceManagementError::unsupported_service(service_key)),
    }
}

pub async fn get_resource_detail(
    config: &AppConfig,
    request: &ResourceDetailRequest,
) -> Result<ResourceDetail, ServiceManagementError> {
    let inventory = list_resources(config, &request.service_key).await?;
    let summary = inventory
        .resources
        .into_iter()
        .find(|resource| resource.id == request.resource_id)
        .ok_or_else(|| {
            ServiceManagementError::not_found(
                request.service_key.clone(),
                "resource_detail",
                format!("Resource `{}` was not found.", request.resource_id),
            )
        })?;

    Ok(ResourceDetail {
        metadata: serde_json::to_value(&summary).map_err(|err| {
            ServiceManagementError::serialization_error(
                request.service_key.clone(),
                "serialize_resource_detail",
                err,
            )
        })?,
        summary,
        relationships: Vec::new(),
    })
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.service_key.as_str() {
        "s3" => s3::execute_action(config, request).await,
        "dynamodb" => dynamodb::execute_action(config, request).await,
        service_key
            if SERVICE_KEYS.contains(&service_key) && request.action == "refresh_inventory" =>
        {
            Ok(ActionResult {
                changed: false,
                message: format!("Refreshed `{service_key}` inventory."),
                resource_id: None,
            })
        }
        service_key if SERVICE_KEYS.contains(&service_key) => Err(unsupported_action(request)),
        service_key => Err(ServiceManagementError::unsupported_service(service_key)),
    }
}

pub fn read_only_inventory(
    service_key: &str,
    service_label: &str,
    tabs: Vec<crate::service_management::models::ResourceTab>,
    resources: Vec<ResourceSummary>,
) -> ServiceInventory {
    ServiceInventory {
        service_key: service_key.to_owned(),
        service_label: service_label.to_owned(),
        support_level: ServiceSupportLevel::ReadOnly,
        refreshed_at: super::super::actions::now_rfc3339(),
        tabs,
        resources,
        unsupported_operations: vec![
            "create".to_owned(),
            "update".to_owned(),
            "delete".to_owned(),
        ],
    }
}

pub fn local_credentials(config: &AppConfig) -> Credentials {
    Credentials::new(
        config.access_key_id.clone(),
        config.secret_access_key.clone(),
        None,
        None,
        "floci-ui",
    )
}

pub fn format_timestamp(value: Option<&impl fmt::Debug>) -> Option<String> {
    value.map(|date| format!("{date:?}"))
}

pub fn unsupported_action(request: &ServiceActionRequest) -> ServiceManagementError {
    ServiceManagementError::unsupported_operation(
        request.service_key.clone(),
        request.action.clone(),
        format!(
            "`{}` is not enabled for `{}` until Floci reports successful local API support.",
            request.action, request.service_key
        ),
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use url::Url;

    use crate::{config::AppConfig, service_management::models::ServiceActionRequest};

    use super::execute_action;

    #[tokio::test]
    async fn read_only_services_allow_inventory_refresh_action() {
        let request = ServiceActionRequest {
            service_key: "rds".to_owned(),
            action: "refresh_inventory".to_owned(),
            resource_id: None,
            resource_name: None,
            confirmation: None,
            payload: json!({}),
        };

        let result = execute_action(&test_config(), &request)
            .await
            .expect("read-only service refresh should be a no-op success");

        assert!(!result.changed);
        assert_eq!(result.message, "Refreshed `rds` inventory.");
    }

    #[tokio::test]
    async fn read_only_services_still_reject_mutating_actions() {
        let request = ServiceActionRequest {
            service_key: "rds".to_owned(),
            action: "delete_instance".to_owned(),
            resource_id: Some("db-instance/local".to_owned()),
            resource_name: Some("local".to_owned()),
            confirmation: Some("local".to_owned()),
            payload: json!({}),
        };

        let error = execute_action(&test_config(), &request)
            .await
            .expect_err("mutating action should remain unsupported");

        assert_eq!(error.code, "unsupported_operation");
    }

    fn test_config() -> AppConfig {
        AppConfig {
            endpoint_url: Url::parse("http://localhost:4566").expect("test endpoint should parse"),
            region: "us-east-1".to_owned(),
            access_key_id: "test".to_owned(),
            secret_access_key: "test".to_owned(),
        }
    }
}
