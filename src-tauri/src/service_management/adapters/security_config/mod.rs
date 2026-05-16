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

pub mod acm;
pub mod appconfig;
pub mod appconfigdata;
pub mod cognito;
pub mod iam;
pub mod kms;
pub mod secretsmanager;
pub mod ssm;
pub mod sts;

pub const SERVICE_KEYS: &[&str] = &[
    "iam",
    "sts",
    "cognito",
    "kms",
    "secretsmanager",
    "ssm",
    "appconfig",
    "appconfigdata",
    "acm",
];

pub async fn list_resources(
    config: &AppConfig,
    service_key: &str,
) -> Result<ServiceInventory, ServiceManagementError> {
    match service_key {
        "iam" => iam::list_resources(config).await,
        "sts" => sts::list_resources(config).await,
        "cognito" => cognito::list_resources(config).await,
        "kms" => kms::list_resources(config).await,
        "secretsmanager" => secretsmanager::list_resources(config).await,
        "ssm" => ssm::list_resources(config).await,
        "appconfig" => appconfig::list_resources(config).await,
        "appconfigdata" => appconfigdata::list_resources(config).await,
        "acm" => acm::list_resources(config).await,
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
        "iam" => iam::execute_action(config, request).await,
        "cognito" => cognito::execute_action(config, request).await,
        "kms" => kms::execute_action(config, request).await,
        "secretsmanager" => secretsmanager::execute_action(config, request).await,
        "ssm" => ssm::execute_action(config, request).await,
        "appconfig" => appconfig::execute_action(config, request).await,
        "acm" => acm::execute_action(config, request).await,
        service_key
            if SERVICE_KEYS.contains(&service_key) && request.action == "refresh_inventory" =>
        {
            Ok(ActionResult {
                changed: false,
                message: format!("Refreshed `{service_key}` inventory."),
                resource_id: request.resource_id.clone(),
            })
        }
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

pub fn require_json_payload_text(
    request: &ServiceActionRequest,
    field_name: &str,
    default_value: &str,
) -> Result<String, ServiceManagementError> {
    let value =
        optional_payload_text(request, field_name).unwrap_or_else(|| default_value.to_owned());

    serde_json::from_str::<serde_json::Value>(&value).map_err(|err| {
        ServiceManagementError::invalid_input(
            request.service_key.clone(),
            request.action.clone(),
            format!("`{field_name}` must be valid JSON: {err}"),
        )
    })?;

    Ok(value)
}

pub fn resource_name_from_arn(arn: &str) -> &str {
    arn.rsplit([':', '/']).next().unwrap_or(arn)
}

pub fn redacted_value_marker() -> String {
    "Hidden until explicit reveal; value is never included in inventory.".to_owned()
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
    async fn read_only_security_services_allow_inventory_refresh_action() {
        let request = ServiceActionRequest {
            service_key: "sts".to_owned(),
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
        assert_eq!(result.message, "Refreshed `sts` inventory.");
    }

    #[tokio::test]
    async fn read_only_security_services_still_reject_mutating_actions() {
        let request = ServiceActionRequest {
            service_key: "sts".to_owned(),
            action: "delete_session".to_owned(),
            resource_id: Some("session/current".to_owned()),
            resource_name: Some("current".to_owned()),
            confirmation: Some("current".to_owned()),
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
