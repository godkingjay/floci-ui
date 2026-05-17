use std::collections::BTreeMap;

use aws_sdk_apigateway::types::IntegrationType;

use crate::{
    config::AppConfig,
    service_management::{
        actions::{require_name_payload, require_typed_confirmation},
        adapters::network_observability::{
            format_timestamp, insert_attr, local_credentials, managed_inventory,
            optional_payload_text, redacted_value_marker, require_payload_text,
            require_resource_id, unsupported_action,
        },
        errors::ServiceManagementError,
        models::{
            ActionResult, ResourceSummary, ResourceTab, ServiceActionRequest, ServiceInventory,
        },
    },
};

pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let client = client(config);
    let apis =
        client.get_rest_apis().send().await.map_err(|err| {
            ServiceManagementError::client_error("apigateway", "get_rest_apis", err)
        })?;
    let mut resources = Vec::new();

    for api in apis.items() {
        let Some(api_id) = api.id() else {
            continue;
        };
        let api_name = api.name().unwrap_or(api_id);
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "description", api.description());
        insert_attr(&mut attributes, "version", api.version());
        insert_attr(&mut attributes, "root_resource_id", api.root_resource_id());
        insert_attr(
            &mut attributes,
            "disable_execute_api_endpoint",
            Some(api.disable_execute_api_endpoint()),
        );
        if let Some(api_status) = api.api_status() {
            insert_attr(
                &mut attributes,
                "api_status",
                Some(format!("{api_status:?}")),
            );
        }

        resources.push(ResourceSummary {
            id: format!("rest-api/{api_id}"),
            name: api_name.to_owned(),
            kind: "rest-api".to_owned(),
            status: api
                .api_status()
                .map_or_else(|| "available".to_owned(), |status| format!("{status:?}")),
            created_at: format_timestamp(api.created_date()),
            updated_at: None,
            tags: api
                .tags()
                .map(|tags| {
                    tags.iter()
                        .map(|(key, value)| (key.clone(), value.clone()))
                        .collect()
                })
                .unwrap_or_default(),
            attributes,
        });

        load_child_resources(&client, api_id, &mut resources).await;
        load_deployments(&client, api_id, &mut resources).await;
        load_stages(&client, api_id, &mut resources).await;
    }

    Ok(managed_inventory(
        "apigateway",
        "API Gateway",
        tabs(),
        resources,
        vec!["api_keys_redacted".to_owned()],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_rest_api" => {
            let api_name = require_name_payload(request, "api_name")?;
            let output = client(config)
                .create_rest_api()
                .name(&api_name)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("apigateway", "create_rest_api", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created REST API `{api_name}`."),
                resource_id: output.id().map(|api_id| format!("rest-api/{api_id}")),
            })
        }
        "create_resource" => {
            let (api_id, parent_id) = selected_api_resource(request)?;
            let path_part = require_payload_text(request, "path_part")?;
            let output = client(config)
                .create_resource()
                .rest_api_id(api_id)
                .parent_id(parent_id)
                .path_part(&path_part)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("apigateway", "create_resource", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created REST resource `{path_part}`."),
                resource_id: output
                    .id()
                    .map(|resource_id| format!("resource/{api_id}/{resource_id}")),
            })
        }
        "put_method" => {
            let (api_id, resource_id) = selected_api_resource(request)?;
            let http_method = optional_payload_text(request, "http_method")
                .unwrap_or_else(|| "GET".to_owned())
                .to_ascii_uppercase();

            client(config)
                .put_method()
                .rest_api_id(api_id)
                .resource_id(resource_id)
                .http_method(&http_method)
                .authorization_type("NONE")
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("apigateway", "put_method", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Added `{http_method}` method."),
                resource_id: Some(format!("method/{api_id}/{resource_id}/{http_method}")),
            })
        }
        "put_integration" => {
            let (api_id, resource_id, http_method) = selected_method(request)?;
            let integration_uri = require_payload_text(request, "integration_uri")?;
            let integration_method = optional_payload_text(request, "integration_method")
                .unwrap_or_else(|| "POST".to_owned())
                .to_ascii_uppercase();

            client(config)
                .put_integration()
                .rest_api_id(api_id)
                .resource_id(resource_id)
                .http_method(http_method)
                .r#type(IntegrationType::HttpProxy)
                .integration_http_method(&integration_method)
                .uri(&integration_uri)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("apigateway", "put_integration", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Added `{integration_method}` integration."),
                resource_id: Some(format!("integration/{api_id}/{resource_id}/{http_method}")),
            })
        }
        "create_deployment" => {
            let api_id = require_resource_id(request, "rest-api/")?;
            let stage_name =
                optional_payload_text(request, "stage_name").unwrap_or_else(|| "local".to_owned());
            let output = client(config)
                .create_deployment()
                .rest_api_id(api_id)
                .stage_name(&stage_name)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("apigateway", "create_deployment", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deployed REST API to stage `{stage_name}`."),
                resource_id: output
                    .id()
                    .map(|deployment_id| format!("deployment/{api_id}/{deployment_id}")),
            })
        }
        "delete_rest_api" => {
            let api_name = require_typed_confirmation(request)?;
            let api_id = require_resource_id(request, "rest-api/")?;

            client(config)
                .delete_rest_api()
                .rest_api_id(api_id)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("apigateway", "delete_rest_api", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted REST API `{api_name}`."),
                resource_id: Some(format!("rest-api/{api_id}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "rest-apis".to_owned(),
            label: "REST APIs".to_owned(),
            kinds: vec!["rest-api".to_owned()],
            empty_message: "No REST APIs were found in the local emulator.".to_owned(),
        },
        ResourceTab {
            key: "resources".to_owned(),
            label: "Resources".to_owned(),
            kinds: vec!["resource".to_owned()],
            empty_message: "No REST API resources are loaded.".to_owned(),
        },
        ResourceTab {
            key: "methods".to_owned(),
            label: "Methods".to_owned(),
            kinds: vec!["method".to_owned()],
            empty_message: "No REST API methods are loaded.".to_owned(),
        },
        ResourceTab {
            key: "integrations".to_owned(),
            label: "Integrations".to_owned(),
            kinds: vec!["integration".to_owned()],
            empty_message: "No REST API integrations are loaded.".to_owned(),
        },
        ResourceTab {
            key: "deployments".to_owned(),
            label: "Deployments".to_owned(),
            kinds: vec!["deployment".to_owned()],
            empty_message: "No REST API deployments are loaded.".to_owned(),
        },
        ResourceTab {
            key: "stages".to_owned(),
            label: "Stages".to_owned(),
            kinds: vec!["stage".to_owned()],
            empty_message: "No REST API stages are loaded.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_apigateway::Client {
    let sdk_config = aws_sdk_apigateway::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_apigateway::config::Region::new(
            config.region.clone(),
        ))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_apigateway::Client::from_conf(sdk_config)
}

async fn load_child_resources(
    client: &aws_sdk_apigateway::Client,
    api_id: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client.get_resources().rest_api_id(api_id).send().await else {
        return;
    };

    for resource in output.items() {
        let Some(resource_id) = resource.id() else {
            continue;
        };
        let path = resource.path().unwrap_or(resource_id);
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "api_id", Some(api_id));
        insert_attr(&mut attributes, "parent_id", resource.parent_id());
        insert_attr(&mut attributes, "path_part", resource.path_part());
        insert_attr(&mut attributes, "path", resource.path());

        resources.push(ResourceSummary {
            id: format!("resource/{api_id}/{resource_id}"),
            name: path.to_owned(),
            kind: "resource".to_owned(),
            status: "available".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes: attributes.clone(),
        });

        for (http_method, method) in resource.resource_methods().into_iter().flatten() {
            let mut method_attributes = BTreeMap::from([
                ("api_id".to_owned(), api_id.to_owned()),
                ("resource_id".to_owned(), resource_id.to_owned()),
                ("path".to_owned(), path.to_owned()),
            ]);
            insert_attr(
                &mut method_attributes,
                "authorization_type",
                method.authorization_type(),
            );
            insert_attr(
                &mut method_attributes,
                "api_key_required",
                method.api_key_required(),
            );

            resources.push(ResourceSummary {
                id: format!("method/{api_id}/{resource_id}/{http_method}"),
                name: format!("{http_method} {path}"),
                kind: "method".to_owned(),
                status: method.authorization_type().unwrap_or("NONE").to_owned(),
                created_at: None,
                updated_at: None,
                tags: BTreeMap::new(),
                attributes: method_attributes,
            });

            if let Some(integration) = method.method_integration() {
                let mut integration_attributes = BTreeMap::from([
                    ("api_id".to_owned(), api_id.to_owned()),
                    ("resource_id".to_owned(), resource_id.to_owned()),
                    ("path".to_owned(), path.to_owned()),
                    ("http_method".to_owned(), http_method.to_owned()),
                ]);
                if let Some(integration_type) = integration.r#type() {
                    insert_attr(
                        &mut integration_attributes,
                        "integration_type",
                        Some(format!("{integration_type:?}")),
                    );
                }
                insert_attr(
                    &mut integration_attributes,
                    "integration_method",
                    integration.http_method(),
                );
                insert_attr(&mut integration_attributes, "uri", integration.uri());
                if integration.credentials().is_some() {
                    insert_attr(
                        &mut integration_attributes,
                        "credentials",
                        Some(redacted_value_marker()),
                    );
                }

                resources.push(ResourceSummary {
                    id: format!("integration/{api_id}/{resource_id}/{http_method}"),
                    name: format!("{http_method} {path} integration"),
                    kind: "integration".to_owned(),
                    status: integration.r#type().map_or_else(
                        || "configured".to_owned(),
                        |integration_type| format!("{integration_type:?}"),
                    ),
                    created_at: None,
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes: integration_attributes,
                });
            }
        }
    }
}

async fn load_deployments(
    client: &aws_sdk_apigateway::Client,
    api_id: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client.get_deployments().rest_api_id(api_id).send().await else {
        return;
    };

    for deployment in output.items() {
        let Some(deployment_id) = deployment.id() else {
            continue;
        };
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "api_id", Some(api_id));
        insert_attr(&mut attributes, "description", deployment.description());

        resources.push(ResourceSummary {
            id: format!("deployment/{api_id}/{deployment_id}"),
            name: deployment_id.to_owned(),
            kind: "deployment".to_owned(),
            status: "deployed".to_owned(),
            created_at: format_timestamp(deployment.created_date()),
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

async fn load_stages(
    client: &aws_sdk_apigateway::Client,
    api_id: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client.get_stages().rest_api_id(api_id).send().await else {
        return;
    };

    for stage in output.item() {
        let Some(stage_name) = stage.stage_name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "api_id", Some(api_id));
        insert_attr(&mut attributes, "deployment_id", stage.deployment_id());
        insert_attr(&mut attributes, "description", stage.description());

        resources.push(ResourceSummary {
            id: format!("stage/{api_id}/{stage_name}"),
            name: stage_name.to_owned(),
            kind: "stage".to_owned(),
            status: "deployed".to_owned(),
            created_at: None,
            updated_at: format_timestamp(stage.last_updated_date()),
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

fn selected_api_resource(
    request: &ServiceActionRequest,
) -> Result<(&str, &str), ServiceManagementError> {
    let selected = require_resource_id(request, "resource/")?;
    let mut parts = selected.splitn(2, '/');
    let api_id = parts.next().unwrap_or_default();
    let resource_id = parts.next().unwrap_or_default();

    if api_id.is_empty() || resource_id.is_empty() {
        return Err(ServiceManagementError::invalid_input(
            request.service_key.clone(),
            request.action.clone(),
            "Select a REST API resource before running this action.",
        ));
    }

    Ok((api_id, resource_id))
}

fn selected_method(
    request: &ServiceActionRequest,
) -> Result<(&str, &str, &str), ServiceManagementError> {
    let selected = require_resource_id(request, "method/")?;
    let mut parts = selected.splitn(3, '/');
    let api_id = parts.next().unwrap_or_default();
    let resource_id = parts.next().unwrap_or_default();
    let http_method = parts.next().unwrap_or_default();

    if api_id.is_empty() || resource_id.is_empty() || http_method.is_empty() {
        return Err(ServiceManagementError::invalid_input(
            request.service_key.clone(),
            request.action.clone(),
            "Select a REST API method before running this action.",
        ));
    }

    Ok((api_id, resource_id, http_method))
}
