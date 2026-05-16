use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{
    config::AppConfig,
    service_management::{
        adapters::{compute_build, data_storage, messaging_events, security_config},
        errors::ServiceManagementError,
        models::{
            ActionResult, ResourceDetail, ResourceDetailRequest, ServiceActionRequest,
            ServiceInventory,
        },
    },
};

pub async fn list_resources(
    config: &AppConfig,
    service_key: &str,
) -> Result<ServiceInventory, ServiceManagementError> {
    let service_key = normalize_service_key(service_key, "list_resources")?;

    if data_storage::SERVICE_KEYS.contains(&service_key.as_str()) {
        return data_storage::list_resources(config, &service_key).await;
    }

    if messaging_events::SERVICE_KEYS.contains(&service_key.as_str()) {
        return messaging_events::list_resources(config, &service_key).await;
    }

    if compute_build::SERVICE_KEYS.contains(&service_key.as_str()) {
        return compute_build::list_resources(config, &service_key).await;
    }

    if security_config::SERVICE_KEYS.contains(&service_key.as_str()) {
        return security_config::list_resources(config, &service_key).await;
    }

    crate::service_management::registry::unsupported_inventory(&service_key)
}

pub async fn get_resource_detail(
    config: &AppConfig,
    request: &ResourceDetailRequest,
) -> Result<ResourceDetail, ServiceManagementError> {
    let service_key = normalize_service_key(&request.service_key, "resource_detail")?;

    if data_storage::SERVICE_KEYS.contains(&service_key.as_str()) {
        let request = ResourceDetailRequest {
            service_key,
            resource_id: request.resource_id.clone(),
        };

        return data_storage::get_resource_detail(config, &request).await;
    }

    if messaging_events::SERVICE_KEYS.contains(&service_key.as_str()) {
        let request = ResourceDetailRequest {
            service_key,
            resource_id: request.resource_id.clone(),
        };

        return messaging_events::get_resource_detail(config, &request).await;
    }

    if compute_build::SERVICE_KEYS.contains(&service_key.as_str()) {
        let request = ResourceDetailRequest {
            service_key,
            resource_id: request.resource_id.clone(),
        };

        return compute_build::get_resource_detail(config, &request).await;
    }

    if security_config::SERVICE_KEYS.contains(&service_key.as_str()) {
        let request = ResourceDetailRequest {
            service_key,
            resource_id: request.resource_id.clone(),
        };

        return security_config::get_resource_detail(config, &request).await;
    }

    if crate::service_management::registry::descriptor(&service_key).is_some() {
        return Err(ServiceManagementError::unsupported_operation(
            service_key,
            "resource_detail",
            "Resource detail is not available until this service adapter is implemented.",
        ));
    }

    Err(ServiceManagementError::unsupported_service(service_key))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    let service_key = normalize_service_key(&request.service_key, "execute_action")?;
    let action = request.action.trim();

    if action.is_empty() {
        return Err(ServiceManagementError::invalid_input(
            service_key,
            "execute_action",
            "Action cannot be empty.",
        ));
    }

    if data_storage::SERVICE_KEYS.contains(&service_key.as_str()) {
        let request = ServiceActionRequest {
            service_key,
            action: action.to_owned(),
            resource_id: request.resource_id.clone(),
            resource_name: request.resource_name.clone(),
            confirmation: request.confirmation.clone(),
            payload: request.payload.clone(),
        };

        return data_storage::execute_action(config, &request).await;
    }

    if messaging_events::SERVICE_KEYS.contains(&service_key.as_str()) {
        let request = ServiceActionRequest {
            service_key,
            action: action.to_owned(),
            resource_id: request.resource_id.clone(),
            resource_name: request.resource_name.clone(),
            confirmation: request.confirmation.clone(),
            payload: request.payload.clone(),
        };

        return messaging_events::execute_action(config, &request).await;
    }

    if compute_build::SERVICE_KEYS.contains(&service_key.as_str()) {
        let request = ServiceActionRequest {
            service_key,
            action: action.to_owned(),
            resource_id: request.resource_id.clone(),
            resource_name: request.resource_name.clone(),
            confirmation: request.confirmation.clone(),
            payload: request.payload.clone(),
        };

        return compute_build::execute_action(config, &request).await;
    }

    if security_config::SERVICE_KEYS.contains(&service_key.as_str()) {
        let request = ServiceActionRequest {
            service_key,
            action: action.to_owned(),
            resource_id: request.resource_id.clone(),
            resource_name: request.resource_name.clone(),
            confirmation: request.confirmation.clone(),
            payload: request.payload.clone(),
        };

        return security_config::execute_action(config, &request).await;
    }

    if crate::service_management::registry::descriptor(&service_key).is_some() {
        return Err(ServiceManagementError::unsupported_operation(
            service_key,
            action,
            "Actions are not available until this service adapter is implemented.",
        ));
    }

    Err(ServiceManagementError::unsupported_service(service_key))
}

fn normalize_service_key(
    service_key: &str,
    operation: &str,
) -> Result<String, ServiceManagementError> {
    let trimmed = service_key.trim();

    if trimmed.is_empty() {
        return Err(ServiceManagementError::invalid_input(
            service_key,
            operation,
            "Service key cannot be empty.",
        ));
    }

    if !trimmed
        .chars()
        .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
    {
        return Err(ServiceManagementError::invalid_input(
            trimmed,
            operation,
            "Service key must use lowercase ASCII letters and digits.",
        ));
    }

    Ok(trimmed.to_owned())
}

pub fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

pub fn require_name_payload(
    request: &ServiceActionRequest,
    field_name: &str,
) -> Result<String, ServiceManagementError> {
    let Some(name) = request
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
    let name = name.trim();

    if name.is_empty() {
        return Err(ServiceManagementError::invalid_input(
            request.service_key.clone(),
            request.action.clone(),
            format!("`{field_name}` cannot be empty."),
        ));
    }

    Ok(name.to_owned())
}

pub fn require_typed_confirmation(
    request: &ServiceActionRequest,
) -> Result<String, ServiceManagementError> {
    let expected = request
        .resource_name
        .as_deref()
        .or(request.resource_id.as_deref())
        .ok_or_else(|| {
            ServiceManagementError::invalid_input(
                request.service_key.clone(),
                request.action.clone(),
                "A resource name is required for typed confirmation.",
            )
        })?;
    let actual = request.confirmation.as_deref().unwrap_or_default().trim();

    if actual != expected {
        return Err(ServiceManagementError::invalid_input(
            request.service_key.clone(),
            request.action.clone(),
            format!("Type `{expected}` to confirm this action."),
        ));
    }

    Ok(expected.to_owned())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use url::Url;

    use crate::{
        config::AppConfig,
        service_management::models::{
            ResourceDetailRequest, ServiceActionRequest, ServiceSupportLevel,
        },
    };

    use super::{
        execute_action, get_resource_detail, list_resources, require_name_payload,
        require_typed_confirmation,
    };

    #[test]
    fn requires_create_name_payload() {
        let request = ServiceActionRequest {
            service_key: "s3".to_owned(),
            action: "create_bucket".to_owned(),
            resource_id: None,
            resource_name: None,
            confirmation: None,
            payload: json!({}),
        };

        let error = require_name_payload(&request, "bucket_name").expect_err("name is required");

        assert_eq!(error.code, "invalid_input");
    }

    #[test]
    fn requires_typed_delete_confirmation() {
        let request = ServiceActionRequest {
            service_key: "dynamodb".to_owned(),
            action: "delete_table".to_owned(),
            resource_id: Some("table/orders".to_owned()),
            resource_name: Some("orders".to_owned()),
            confirmation: Some("wrong".to_owned()),
            payload: json!({}),
        };

        let error = require_typed_confirmation(&request).expect_err("confirmation should match");

        assert_eq!(error.code, "invalid_input");
    }

    #[tokio::test]
    async fn returns_unsupported_service_for_unknown_inventory_key() {
        let config = test_config();

        let error = list_resources(&config, "notarealservice")
            .await
            .expect_err("unknown service should be rejected");

        assert_eq!(error.code, "unsupported_service");
    }

    #[tokio::test]
    async fn returns_unsupported_inventory_for_known_service_without_adapter() {
        let config = test_config();

        let inventory = list_resources(&config, "apigateway")
            .await
            .expect("known security service should route to its adapter");

        assert_eq!(inventory.service_key, "apigateway");
        assert_eq!(inventory.support_level, ServiceSupportLevel::Unsupported);
        assert!(inventory.resources.is_empty());
        assert!(!inventory.tabs.is_empty());
    }

    #[tokio::test]
    async fn returns_unsupported_operation_for_known_service_without_adapter() {
        let config = test_config();
        let request = ServiceActionRequest {
            service_key: "apigateway".to_owned(),
            action: "create_function".to_owned(),
            resource_id: None,
            resource_name: None,
            confirmation: None,
            payload: json!({}),
        };

        let error = execute_action(&config, &request)
            .await
            .expect_err("known service should reject unsupported operation");

        assert_eq!(error.code, "unsupported_operation");
    }

    #[tokio::test]
    async fn rejects_invalid_service_key_before_detail_lookup() {
        let config = test_config();
        let request = ResourceDetailRequest {
            service_key: "S3!".to_owned(),
            resource_id: "bucket/example".to_owned(),
        };

        let error = get_resource_detail(&config, &request)
            .await
            .expect_err("invalid service key should fail validation");

        assert_eq!(error.code, "invalid_input");
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
