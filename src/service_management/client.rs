use serde_json::json;

use crate::{
    commands::invoke_command_with_args,
    service_management::models::{
        ActionResult, ResourceDetail, ResourceDetailRequest, ServiceActionRequest,
        ServiceInventory, ServiceInventoryRequest,
    },
};

pub async fn service_inventory(service_key: String) -> Result<ServiceInventory, String> {
    invoke_command_with_args(
        "service_inventory",
        &json!({
            "request": ServiceInventoryRequest { service_key },
        }),
    )
    .await
}

pub async fn service_resource_detail(
    service_key: String,
    resource_id: String,
) -> Result<ResourceDetail, String> {
    invoke_command_with_args(
        "service_resource_detail",
        &json!({
            "request": ResourceDetailRequest {
                service_key,
                resource_id,
            },
        }),
    )
    .await
}

pub async fn service_execute_action(request: ServiceActionRequest) -> Result<ActionResult, String> {
    invoke_command_with_args(
        "service_execute_action",
        &json!({
            "request": request,
        }),
    )
    .await
}
