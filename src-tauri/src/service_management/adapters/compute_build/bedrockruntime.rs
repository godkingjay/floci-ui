use std::collections::BTreeMap;

use aws_sdk_bedrockruntime::primitives::Blob;

use crate::{
    config::AppConfig,
    service_management::{
        adapters::compute_build::{
            local_credentials, managed_inventory, optional_payload_text, require_json_payload_text,
            require_payload_text, unsupported_action,
        },
        errors::ServiceManagementError,
        models::{
            ActionResult, ResourceSummary, ResourceTab, ServiceActionRequest, ServiceInventory,
        },
    },
};

pub async fn list_resources(
    _config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let resources = vec![
        ResourceSummary {
            id: "model/local-bedrock-runtime".to_owned(),
            name: "local-bedrock-runtime".to_owned(),
            kind: "model".to_owned(),
            status: "configured".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes: BTreeMap::from([
                (
                    "execution_context".to_owned(),
                    "Local Floci endpoint only".to_owned(),
                ),
                (
                    "inventory_note".to_owned(),
                    "Bedrock Runtime is action-first; model catalog is supplied by Bedrock APIs."
                        .to_owned(),
                ),
            ]),
        },
        ResourceSummary {
            id: "request-template/json-invoke".to_owned(),
            name: "JSON invoke template".to_owned(),
            kind: "request-template".to_owned(),
            status: "available".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes: BTreeMap::from([(
                "template".to_owned(),
                r#"{"prompt":"hello from floci-ui"}"#.to_owned(),
            )]),
        },
        ResourceSummary {
            id: "response-preview/latest".to_owned(),
            name: "Latest response preview".to_owned(),
            kind: "response-preview".to_owned(),
            status: "empty".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes: BTreeMap::from([(
                "large_outputs".to_owned(),
                "Invocation responses are summarized instead of streamed.".to_owned(),
            )]),
        },
    ];

    Ok(managed_inventory(
        "bedrockruntime",
        "Bedrock Runtime",
        tabs(),
        resources,
        vec!["stream_invoke_model".to_owned()],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "invoke_model" => {
            let model_id = optional_payload_text(request, "model_id")
                .or_else(|| {
                    request
                        .resource_id
                        .as_deref()?
                        .strip_prefix("model/")
                        .map(ToOwned::to_owned)
                })
                .ok_or_else(|| {
                    ServiceManagementError::invalid_input(
                        request.service_key.clone(),
                        request.action.clone(),
                        "`model_id` is required.",
                    )
                })?;
            let body = require_json_payload_text(request, "body", "{}")?;
            let content_type = optional_payload_text(request, "content_type")
                .unwrap_or_else(|| "application/json".to_owned());
            let accept =
                optional_payload_text(request, "accept").unwrap_or_else(|| content_type.clone());
            let output = client(config)
                .invoke_model()
                .model_id(&model_id)
                .content_type(content_type)
                .accept(accept)
                .body(Blob::new(body.into_bytes()))
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("bedrockruntime", "invoke_model", err)
                })?;
            let response = String::from_utf8_lossy(output.body().as_ref()).to_string();

            Ok(ActionResult {
                changed: false,
                message: format!(
                    "Invoked Bedrock Runtime model `{model_id}`. Response preview: {}",
                    response.chars().take(240).collect::<String>()
                ),
                resource_id: Some(format!("model/{model_id}")),
            })
        }
        "validate_request_template" => {
            let _ = require_json_payload_text(request, "body", "{}")?;
            let template = require_payload_text(request, "body")?;
            Ok(ActionResult {
                changed: false,
                message: format!(
                    "Validated Bedrock Runtime request template ({} bytes).",
                    template.len()
                ),
                resource_id: request.resource_id.clone(),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        tab(
            "models",
            "Models",
            &["model"],
            "No local Bedrock Runtime metadata is loaded.",
        ),
        tab(
            "invocations",
            "Invocations",
            &["invocation"],
            "No invocation summaries are loaded.",
        ),
        tab(
            "request-template",
            "Request Template",
            &["request-template"],
            "No request templates are loaded.",
        ),
        tab(
            "response-preview",
            "Response Preview",
            &["response-preview"],
            "No response preview is available.",
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

fn client(config: &AppConfig) -> aws_sdk_bedrockruntime::Client {
    let sdk_config = aws_sdk_bedrockruntime::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_bedrockruntime::config::Region::new(
            config.region.clone(),
        ))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_bedrockruntime::Client::from_conf(sdk_config)
}
