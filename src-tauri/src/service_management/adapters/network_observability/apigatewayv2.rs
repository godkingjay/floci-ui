use std::collections::BTreeMap;

use aws_sdk_apigatewayv2::types::{IntegrationType, ProtocolType};

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
    let apis = client
        .get_apis()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("apigatewayv2", "get_apis", err))?;
    let mut resources = Vec::new();

    for api in apis.items() {
        let Some(api_id) = api.api_id() else {
            continue;
        };
        let api_name = api.name().unwrap_or(api_id);
        let mut attributes = BTreeMap::new();

        if let Some(protocol_type) = api.protocol_type() {
            insert_attr(
                &mut attributes,
                "protocol_type",
                Some(protocol_type.as_str()),
            );
        }
        insert_attr(&mut attributes, "api_endpoint", api.api_endpoint());
        insert_attr(&mut attributes, "version", api.version());

        resources.push(ResourceSummary {
            id: format!("api/{api_id}"),
            name: api_name.to_owned(),
            kind: "api".to_owned(),
            status: api.protocol_type().map_or_else(
                || "available".to_owned(),
                |protocol_type| protocol_type.as_str().to_owned(),
            ),
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

        load_routes(&client, api_id, &mut resources).await;
        load_integrations(&client, api_id, &mut resources).await;
        load_deployments(&client, api_id, &mut resources).await;
        load_stages(&client, api_id, &mut resources).await;
        load_authorizers(&client, api_id, &mut resources).await;
    }

    Ok(managed_inventory(
        "apigatewayv2",
        "API Gateway v2",
        tabs(),
        resources,
        vec!["authorizer_identity_redacted".to_owned()],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_api" => {
            let api_name = require_name_payload(request, "api_name")?;
            let protocol_type = ProtocolType::from(
                optional_payload_text(request, "protocol_type")
                    .unwrap_or_else(|| "HTTP".to_owned())
                    .to_ascii_uppercase()
                    .as_str(),
            );
            let output = client(config)
                .create_api()
                .name(&api_name)
                .protocol_type(protocol_type)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("apigatewayv2", "create_api", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created API Gateway v2 API `{api_name}`."),
                resource_id: output.api_id().map(|api_id| format!("api/{api_id}")),
            })
        }
        "create_route" => {
            let api_id = require_resource_id(request, "api/")?;
            let route_key = require_payload_text(request, "route_key")?;
            let mut builder = client(config)
                .create_route()
                .api_id(api_id)
                .route_key(&route_key);
            if let Some(target) = optional_payload_text(request, "target") {
                builder = builder.target(target);
            }
            let output = builder.send().await.map_err(|err| {
                ServiceManagementError::client_error("apigatewayv2", "create_route", err)
            })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created API Gateway route `{route_key}`."),
                resource_id: output
                    .route_id()
                    .map(|route_id| format!("route/{api_id}/{route_id}")),
            })
        }
        "create_integration" => {
            let api_id = require_resource_id(request, "api/")?;
            let integration_uri = require_payload_text(request, "integration_uri")?;
            let integration_method = optional_payload_text(request, "integration_method")
                .unwrap_or_else(|| "POST".to_owned())
                .to_ascii_uppercase();
            let output = client(config)
                .create_integration()
                .api_id(api_id)
                .integration_type(IntegrationType::HttpProxy)
                .integration_method(&integration_method)
                .integration_uri(&integration_uri)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("apigatewayv2", "create_integration", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Created `{integration_method}` integration."),
                resource_id: output
                    .integration_id()
                    .map(|integration_id| format!("integration/{api_id}/{integration_id}")),
            })
        }
        "create_deployment" => {
            let api_id = require_resource_id(request, "api/")?;
            let output = client(config)
                .create_deployment()
                .api_id(api_id)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("apigatewayv2", "create_deployment", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: "Created API Gateway v2 deployment.".to_owned(),
                resource_id: output
                    .deployment_id()
                    .map(|deployment_id| format!("deployment/{api_id}/{deployment_id}")),
            })
        }
        "delete_api" => {
            let api_name = require_typed_confirmation(request)?;
            let api_id = require_resource_id(request, "api/")?;

            client(config)
                .delete_api()
                .api_id(api_id)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("apigatewayv2", "delete_api", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!("Deleted API Gateway v2 API `{api_name}`."),
                resource_id: Some(format!("api/{api_id}")),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        ResourceTab {
            key: "apis".to_owned(),
            label: "APIs".to_owned(),
            kinds: vec!["api".to_owned()],
            empty_message: "No API Gateway v2 APIs were found in the local emulator.".to_owned(),
        },
        ResourceTab {
            key: "routes".to_owned(),
            label: "Routes".to_owned(),
            kinds: vec!["route".to_owned()],
            empty_message: "No API Gateway v2 routes are loaded.".to_owned(),
        },
        ResourceTab {
            key: "integrations".to_owned(),
            label: "Integrations".to_owned(),
            kinds: vec!["integration".to_owned()],
            empty_message: "No API Gateway v2 integrations are loaded.".to_owned(),
        },
        ResourceTab {
            key: "deployments".to_owned(),
            label: "Deployments".to_owned(),
            kinds: vec!["deployment".to_owned()],
            empty_message: "No API Gateway v2 deployments are loaded.".to_owned(),
        },
        ResourceTab {
            key: "stages".to_owned(),
            label: "Stages".to_owned(),
            kinds: vec!["stage".to_owned()],
            empty_message: "No API Gateway v2 stages are loaded.".to_owned(),
        },
        ResourceTab {
            key: "authorizers".to_owned(),
            label: "Authorizers".to_owned(),
            kinds: vec!["authorizer".to_owned()],
            empty_message: "No API Gateway v2 authorizers are loaded.".to_owned(),
        },
    ]
}

fn client(config: &AppConfig) -> aws_sdk_apigatewayv2::Client {
    let sdk_config = aws_sdk_apigatewayv2::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_apigatewayv2::config::Region::new(
            config.region.clone(),
        ))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_apigatewayv2::Client::from_conf(sdk_config)
}

async fn load_routes(
    client: &aws_sdk_apigatewayv2::Client,
    api_id: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client.get_routes().api_id(api_id).send().await else {
        return;
    };

    for route in output.items() {
        let Some(route_id) = route.route_id() else {
            continue;
        };
        let route_key = route.route_key().unwrap_or(route_id);
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "api_id", Some(api_id));
        insert_attr(&mut attributes, "route_key", route.route_key());
        insert_attr(&mut attributes, "target", route.target());
        insert_attr(&mut attributes, "authorizer_id", route.authorizer_id());
        if let Some(authorization_type) = route.authorization_type() {
            insert_attr(
                &mut attributes,
                "authorization_type",
                Some(authorization_type.as_str()),
            );
        }

        resources.push(ResourceSummary {
            id: format!("route/{api_id}/{route_id}"),
            name: route_key.to_owned(),
            kind: "route".to_owned(),
            status: route.authorization_type().map_or_else(
                || "NONE".to_owned(),
                |authorization_type| authorization_type.as_str().to_owned(),
            ),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

async fn load_integrations(
    client: &aws_sdk_apigatewayv2::Client,
    api_id: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client.get_integrations().api_id(api_id).send().await else {
        return;
    };

    for integration in output.items() {
        let Some(integration_id) = integration.integration_id() else {
            continue;
        };
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "api_id", Some(api_id));
        insert_attr(
            &mut attributes,
            "integration_method",
            integration.integration_method(),
        );
        insert_attr(
            &mut attributes,
            "integration_uri",
            integration.integration_uri(),
        );
        if integration.credentials_arn().is_some() {
            insert_attr(
                &mut attributes,
                "credentials_arn",
                Some(redacted_value_marker()),
            );
        }
        if let Some(integration_type) = integration.integration_type() {
            insert_attr(
                &mut attributes,
                "integration_type",
                Some(integration_type.as_str()),
            );
        }

        resources.push(ResourceSummary {
            id: format!("integration/{api_id}/{integration_id}"),
            name: integration
                .integration_uri()
                .unwrap_or(integration_id)
                .to_owned(),
            kind: "integration".to_owned(),
            status: integration.integration_type().map_or_else(
                || "configured".to_owned(),
                |integration_type| integration_type.as_str().to_owned(),
            ),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

async fn load_deployments(
    client: &aws_sdk_apigatewayv2::Client,
    api_id: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client.get_deployments().api_id(api_id).send().await else {
        return;
    };

    for deployment in output.items() {
        let Some(deployment_id) = deployment.deployment_id() else {
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
    client: &aws_sdk_apigatewayv2::Client,
    api_id: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client.get_stages().api_id(api_id).send().await else {
        return;
    };

    for stage in output.items() {
        let Some(stage_name) = stage.stage_name() else {
            continue;
        };
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "api_id", Some(api_id));
        insert_attr(&mut attributes, "deployment_id", stage.deployment_id());
        insert_attr(&mut attributes, "auto_deploy", stage.auto_deploy());

        resources.push(ResourceSummary {
            id: format!("stage/{api_id}/{stage_name}"),
            name: stage_name.to_owned(),
            kind: "stage".to_owned(),
            status: if stage.auto_deploy().unwrap_or_default() {
                "auto-deploy".to_owned()
            } else {
                "manual".to_owned()
            },
            created_at: None,
            updated_at: format_timestamp(stage.last_updated_date()),
            tags: BTreeMap::new(),
            attributes,
        });
    }
}

async fn load_authorizers(
    client: &aws_sdk_apigatewayv2::Client,
    api_id: &str,
    resources: &mut Vec<ResourceSummary>,
) {
    let Ok(output) = client.get_authorizers().api_id(api_id).send().await else {
        return;
    };

    for authorizer in output.items() {
        let Some(authorizer_id) = authorizer.authorizer_id() else {
            continue;
        };
        let authorizer_name = authorizer.name().unwrap_or(authorizer_id);
        let mut attributes = BTreeMap::new();

        insert_attr(&mut attributes, "api_id", Some(api_id));
        if let Some(authorizer_type) = authorizer.authorizer_type() {
            insert_attr(
                &mut attributes,
                "authorizer_type",
                Some(authorizer_type.as_str()),
            );
        }
        insert_attr(
            &mut attributes,
            "identity_sources",
            Some(redacted_value_marker()),
        );

        resources.push(ResourceSummary {
            id: format!("authorizer/{api_id}/{authorizer_id}"),
            name: authorizer_name.to_owned(),
            kind: "authorizer".to_owned(),
            status: authorizer.authorizer_type().map_or_else(
                || "configured".to_owned(),
                |authorizer_type| authorizer_type.as_str().to_owned(),
            ),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });
    }
}
