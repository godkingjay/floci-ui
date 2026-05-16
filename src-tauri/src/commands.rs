use serde::Serialize;
use tauri::State;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{
    AppState,
    models::{DashboardSnapshot, HealthSnapshot},
    service_management::{
        errors::ServiceManagementError,
        models::{
            ActionResult, ResourceDetail, ResourceDetailRequest, ServiceActionRequest,
            ServiceInventory, ServiceInventoryRequest,
        },
    },
};

#[derive(Debug, Serialize)]
pub struct CommandError {
    code: &'static str,
    message: String,
}

impl CommandError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[tauri::command]
pub async fn floci_health(state: State<'_, AppState>) -> Result<HealthSnapshot, CommandError> {
    let snapshot = state.floci().health().await;
    Ok(snapshot)
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn service_catalog(state: State<'_, AppState>) -> Result<DashboardSnapshot, CommandError> {
    let config = state.config();
    let endpoint_url = config.endpoint_url.to_string();

    if endpoint_url.is_empty() {
        return Err(CommandError::new(
            "invalid_endpoint",
            "Floci endpoint URL cannot be empty.",
        ));
    }

    Ok(DashboardSnapshot {
        endpoint_url,
        region: config.region.clone(),
        access_key_id: config.access_key_id.clone(),
        credentials_status: config.credentials_status().to_owned(),
        last_refreshed_at: now_rfc3339(),
        services: crate::services::services(),
    })
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn service_inventory(
    state: State<'_, AppState>,
    request: ServiceInventoryRequest,
) -> Result<ServiceInventory, ServiceManagementError> {
    crate::service_management::list_resources(state.config(), &request.service_key).await
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn service_resource_detail(
    state: State<'_, AppState>,
    request: ResourceDetailRequest,
) -> Result<ResourceDetail, ServiceManagementError> {
    crate::service_management::get_resource_detail(state.config(), &request).await
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn service_execute_action(
    state: State<'_, AppState>,
    request: ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    crate::service_management::execute_action(state.config(), &request).await
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, to_string, to_value};

    use crate::models::DashboardSnapshot;

    #[test]
    fn dashboard_snapshot_serialization_excludes_secret_access_key() {
        let snapshot = DashboardSnapshot {
            endpoint_url: "http://localhost:4566".to_owned(),
            region: "us-east-1".to_owned(),
            access_key_id: "test".to_owned(),
            credentials_status: "Configured".to_owned(),
            last_refreshed_at: "2026-05-15T00:00:00Z".to_owned(),
            services: Vec::new(),
        };

        let value: Value = to_value(&snapshot).expect("snapshot should serialize");
        let serialized = to_string(&snapshot).expect("snapshot should stringify");

        assert!(value.get("secret_access_key").is_none());
        assert!(!serialized.contains("secret_access_key"));
        assert!(!serialized.contains("super-secret"));
    }
}
