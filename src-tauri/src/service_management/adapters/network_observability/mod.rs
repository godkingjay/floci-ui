use std::{collections::BTreeMap, fmt};

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

pub mod apigateway;
pub mod apigatewayv2;
pub mod cloudwatch;
pub mod cloudwatchlogs;
pub mod elbv2;
pub mod route53;
pub mod transfer;

pub const SERVICE_KEYS: &[&str] = &[
    "apigateway",
    "apigatewayv2",
    "elbv2",
    "route53",
    "transfer",
    "cloudwatchlogs",
    "cloudwatch",
];

pub async fn list_resources(
    config: &AppConfig,
    service_key: &str,
) -> Result<ServiceInventory, ServiceManagementError> {
    match service_key {
        "apigateway" => apigateway::list_resources(config).await,
        "apigatewayv2" => apigatewayv2::list_resources(config).await,
        "elbv2" => elbv2::list_resources(config).await,
        "route53" => route53::list_resources(config).await,
        "transfer" => transfer::list_resources(config).await,
        "cloudwatchlogs" => cloudwatchlogs::list_resources(config).await,
        "cloudwatch" => cloudwatch::list_resources(config).await,
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
    if request.action == "refresh_inventory" && SERVICE_KEYS.contains(&request.service_key.as_str())
    {
        return Ok(ActionResult {
            changed: false,
            message: format!("Refreshed `{}` inventory.", request.service_key),
            resource_id: request.resource_id.clone(),
        });
    }

    match request.service_key.as_str() {
        "apigateway" => apigateway::execute_action(config, request).await,
        "apigatewayv2" => apigatewayv2::execute_action(config, request).await,
        "elbv2" => elbv2::execute_action(config, request).await,
        "route53" => route53::execute_action(config, request).await,
        "transfer" => transfer::execute_action(config, request).await,
        "cloudwatchlogs" => cloudwatchlogs::execute_action(config, request).await,
        "cloudwatch" => cloudwatch::execute_action(config, request).await,
        service_key if SERVICE_KEYS.contains(&service_key) => Err(unsupported_action(request)),
        service_key => Err(ServiceManagementError::unsupported_service(service_key)),
    }
}

pub fn managed_inventory(
    service_key: &str,
    service_label: &str,
    tabs: Vec<crate::service_management::models::ResourceTab>,
    resources: Vec<ResourceSummary>,
    unsupported_operations: Vec<String>,
) -> ServiceInventory {
    ServiceInventory {
        service_key: service_key.to_owned(),
        service_label: service_label.to_owned(),
        support_level: ServiceSupportLevel::Managed,
        refreshed_at: super::super::actions::now_rfc3339(),
        tabs,
        resources,
        unsupported_operations,
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

pub fn insert_attr(
    attributes: &mut BTreeMap<String, String>,
    key: &str,
    value: Option<impl ToString>,
) {
    if let Some(value) = value {
        attributes.insert(key.to_owned(), value.to_string());
    }
}

pub fn require_resource_id<'a>(
    request: &'a ServiceActionRequest,
    prefix: &str,
) -> Result<&'a str, ServiceManagementError> {
    request
        .resource_id
        .as_deref()
        .and_then(|value| value.strip_prefix(prefix))
        .ok_or_else(|| {
            ServiceManagementError::invalid_input(
                request.service_key.clone(),
                request.action.clone(),
                "A selected resource is required for this action.",
            )
        })
}

pub fn require_payload_text(
    request: &ServiceActionRequest,
    field_name: &str,
) -> Result<String, ServiceManagementError> {
    let Some(value) = request
        .payload
        .get(field_name)
        .and_then(|value| value.as_str())
    else {
        return Err(ServiceManagementError::invalid_input(
            request.service_key.clone(),
            request.action.clone(),
            format!("`{field_name}` is required."),
        ));
    };
    let value = value.trim();

    if value.is_empty() {
        return Err(ServiceManagementError::invalid_input(
            request.service_key.clone(),
            request.action.clone(),
            format!("`{field_name}` cannot be empty."),
        ));
    }

    Ok(value.to_owned())
}

pub fn optional_payload_text(request: &ServiceActionRequest, field_name: &str) -> Option<String> {
    request
        .payload
        .get(field_name)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn optional_payload_i32(request: &ServiceActionRequest, field_name: &str) -> Option<i32> {
    request.payload.get(field_name).and_then(|value| {
        value
            .as_i64()
            .and_then(|value| i32::try_from(value).ok())
            .or_else(|| value.as_str()?.trim().parse().ok())
    })
}

pub fn redacted_value_marker() -> String {
    "Sensitive identity details are redacted from inventory.".to_owned()
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

pub fn build_error(
    request: &ServiceActionRequest,
    field_name: &str,
    err: impl fmt::Display,
) -> ServiceManagementError {
    ServiceManagementError::invalid_input(
        request.service_key.clone(),
        request.action.clone(),
        format!("`{field_name}` is invalid: {err}"),
    )
}

pub fn resource_name_from_arn(arn: &str) -> &str {
    arn.rsplit([':', '/']).next().unwrap_or(arn)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use url::Url;

    use crate::{config::AppConfig, service_management::models::ServiceActionRequest};

    use super::execute_action;

    #[tokio::test]
    async fn network_observability_services_allow_inventory_refresh_action() {
        let request = ServiceActionRequest {
            service_key: "cloudwatchlogs".to_owned(),
            action: "refresh_inventory".to_owned(),
            resource_id: None,
            resource_name: None,
            confirmation: None,
            payload: json!({}),
        };

        let result = execute_action(&test_config(), &request)
            .await
            .expect("refresh should be a no-op success");

        assert!(!result.changed);
        assert_eq!(result.message, "Refreshed `cloudwatchlogs` inventory.");
    }

    #[tokio::test]
    async fn network_observability_services_reject_unknown_actions() {
        let request = ServiceActionRequest {
            service_key: "cloudwatchlogs".to_owned(),
            action: "delete_everything".to_owned(),
            resource_id: Some("log-group/example".to_owned()),
            resource_name: Some("example".to_owned()),
            confirmation: Some("example".to_owned()),
            payload: json!({}),
        };

        let error = execute_action(&test_config(), &request)
            .await
            .expect_err("unknown action should remain unsupported");

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
