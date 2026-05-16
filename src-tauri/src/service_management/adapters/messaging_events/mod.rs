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

pub mod cloudformation;
pub mod eventbridge;
pub mod kinesis;
pub mod scheduler;
pub mod ses;
pub mod sesv2;
pub mod sns;
pub mod sqs;
pub mod stepfunctions;

pub const SERVICE_KEYS: &[&str] = &[
    "sqs",
    "sns",
    "ses",
    "sesv2",
    "kinesis",
    "eventbridge",
    "scheduler",
    "stepfunctions",
    "cloudformation",
];

pub async fn list_resources(
    config: &AppConfig,
    service_key: &str,
) -> Result<ServiceInventory, ServiceManagementError> {
    match service_key {
        "sqs" => sqs::list_resources(config).await,
        "sns" => sns::list_resources(config).await,
        "ses" => ses::list_resources(config).await,
        "sesv2" => sesv2::list_resources(config).await,
        "kinesis" => kinesis::list_resources(config).await,
        "eventbridge" => eventbridge::list_resources(config).await,
        "scheduler" => scheduler::list_resources(config).await,
        "stepfunctions" => stepfunctions::list_resources(config).await,
        "cloudformation" => cloudformation::list_resources(config).await,
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
        "sqs" => sqs::execute_action(config, request).await,
        "sns" => sns::execute_action(config, request).await,
        "kinesis" => kinesis::execute_action(config, request).await,
        "eventbridge" => eventbridge::execute_action(config, request).await,
        "stepfunctions" => stepfunctions::execute_action(config, request).await,
        "cloudformation" => cloudformation::execute_action(config, request).await,
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
