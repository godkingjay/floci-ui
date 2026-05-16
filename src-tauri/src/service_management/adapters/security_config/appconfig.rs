use std::collections::BTreeMap;

use aws_sdk_appconfig::primitives::Blob;

use crate::{
    config::AppConfig,
    service_management::{
        actions::require_name_payload,
        adapters::security_config::{
            format_timestamp, insert_attr, local_credentials, managed_inventory,
            optional_payload_text, require_json_payload_text, require_resource_id,
            unsupported_action,
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
    let applications = client.list_applications().send().await.map_err(|err| {
        ServiceManagementError::client_error("appconfig", "list_applications", err)
    })?;
    let mut resources = Vec::new();

    for application in applications.items() {
        let Some(application_id) = application.id() else {
            continue;
        };
        let application_name = application.name().unwrap_or(application_id);
        let mut attributes = BTreeMap::new();
        insert_attr(&mut attributes, "description", application.description());

        resources.push(ResourceSummary {
            id: format!("application/{application_id}"),
            name: application_name.to_owned(),
            kind: "application".to_owned(),
            status: "available".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes,
        });

        if let Ok(environments) = client
            .list_environments()
            .application_id(application_id)
            .send()
            .await
        {
            for environment in environments.items() {
                let Some(environment_id) = environment.id() else {
                    continue;
                };
                let mut attributes =
                    BTreeMap::from([("application_id".to_owned(), application_id.to_owned())]);
                insert_attr(&mut attributes, "description", environment.description());
                insert_attr(
                    &mut attributes,
                    "state",
                    environment.state().map(ToString::to_string),
                );

                resources.push(ResourceSummary {
                    id: format!("environment/{application_id}/{environment_id}"),
                    name: environment.name().unwrap_or(environment_id).to_owned(),
                    kind: "environment".to_owned(),
                    status: environment
                        .state()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "available".to_owned()),
                    created_at: None,
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes,
                });

                if let Ok(deployments) = client
                    .list_deployments()
                    .application_id(application_id)
                    .environment_id(environment_id)
                    .send()
                    .await
                {
                    for deployment in deployments.items() {
                        let deployment_number = deployment.deployment_number();
                        resources.push(ResourceSummary {
                            id: format!(
                                "deployment/{application_id}/{environment_id}/{deployment_number}"
                            ),
                            name: format!("#{deployment_number}"),
                            kind: "deployment".to_owned(),
                            status: deployment
                                .state()
                                .map(ToString::to_string)
                                .unwrap_or_else(|| "unknown".to_owned()),
                            created_at: format_timestamp(deployment.started_at()),
                            updated_at: format_timestamp(deployment.completed_at()),
                            tags: BTreeMap::new(),
                            attributes: BTreeMap::from([
                                ("application_id".to_owned(), application_id.to_owned()),
                                ("environment_id".to_owned(), environment_id.to_owned()),
                                (
                                    "configuration_version".to_owned(),
                                    deployment
                                        .configuration_version()
                                        .unwrap_or("unknown")
                                        .to_owned(),
                                ),
                            ]),
                        });
                    }
                }
            }
        }

        if let Ok(profiles) = client
            .list_configuration_profiles()
            .application_id(application_id)
            .send()
            .await
        {
            for profile in profiles.items() {
                let Some(profile_id) = profile.id() else {
                    continue;
                };
                let mut attributes =
                    BTreeMap::from([("application_id".to_owned(), application_id.to_owned())]);
                insert_attr(&mut attributes, "location_uri", profile.location_uri());
                insert_attr(
                    &mut attributes,
                    "type",
                    profile.r#type().map(ToString::to_string),
                );

                resources.push(ResourceSummary {
                    id: format!("configuration-profile/{application_id}/{profile_id}"),
                    name: profile.name().unwrap_or(profile_id).to_owned(),
                    kind: "configuration-profile".to_owned(),
                    status: "available".to_owned(),
                    created_at: None,
                    updated_at: None,
                    tags: BTreeMap::new(),
                    attributes,
                });

                if let Ok(versions) = client
                    .list_hosted_configuration_versions()
                    .application_id(application_id)
                    .configuration_profile_id(profile_id)
                    .send()
                    .await
                {
                    for version in versions.items() {
                        let version_number = version.version_number();
                        resources.push(ResourceSummary {
                            id: format!(
                                "hosted-version/{application_id}/{profile_id}/{version_number}"
                            ),
                            name: format!("v{version_number}"),
                            kind: "hosted-version".to_owned(),
                            status: "available".to_owned(),
                            created_at: None,
                            updated_at: None,
                            tags: BTreeMap::new(),
                            attributes: BTreeMap::from([
                                ("application_id".to_owned(), application_id.to_owned()),
                                ("configuration_profile_id".to_owned(), profile_id.to_owned()),
                                (
                                    "content_type".to_owned(),
                                    version
                                        .content_type()
                                        .unwrap_or("application/json")
                                        .to_owned(),
                                ),
                                ("content".to_owned(), "metadata only".to_owned()),
                            ]),
                        });
                    }
                }
            }
        }
    }

    Ok(managed_inventory(
        "appconfig",
        "AppConfig",
        tabs(),
        resources,
        vec![
            "delete_application".to_owned(),
            "delete_environment".to_owned(),
        ],
    ))
}

pub async fn execute_action(
    config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    match request.action.as_str() {
        "create_application" => {
            let name = require_name_payload(request, "application_name")?;
            let output = client(config)
                .create_application()
                .name(&name)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("appconfig", "create_application", err)
                })?;
            let application_id = output.id().unwrap_or(&name);

            Ok(ActionResult {
                changed: true,
                message: format!("Created AppConfig application `{name}`."),
                resource_id: Some(format!("application/{application_id}")),
            })
        }
        "create_environment" => {
            let name = require_name_payload(request, "environment_name")?;
            let application_id = require_resource_id(request, "application/")?;
            let output = client(config)
                .create_environment()
                .application_id(application_id)
                .name(&name)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("appconfig", "create_environment", err)
                })?;
            let environment_id = output.id().unwrap_or(&name);

            Ok(ActionResult {
                changed: true,
                message: format!("Created AppConfig environment `{name}`."),
                resource_id: Some(format!("environment/{application_id}/{environment_id}")),
            })
        }
        "create_configuration_profile" => {
            let name = require_name_payload(request, "profile_name")?;
            let application_id = require_resource_id(request, "application/")?;
            let location_uri = optional_payload_text(request, "location_uri")
                .unwrap_or_else(|| "hosted".to_owned());
            let output = client(config)
                .create_configuration_profile()
                .application_id(application_id)
                .name(&name)
                .location_uri(location_uri)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error(
                        "appconfig",
                        "create_configuration_profile",
                        err,
                    )
                })?;
            let profile_id = output.id().unwrap_or(&name);

            Ok(ActionResult {
                changed: true,
                message: format!("Created AppConfig configuration profile `{name}`."),
                resource_id: Some(format!(
                    "configuration-profile/{application_id}/{profile_id}"
                )),
            })
        }
        "create_hosted_version" => {
            let content = require_json_payload_text(request, "content", "{}")?;
            let profile_path = require_resource_id(request, "configuration-profile/")?;
            let mut parts = profile_path.splitn(2, '/');
            let application_id = parts.next().unwrap_or_default();
            let profile_id = parts.next().unwrap_or(profile_path);
            let content_type = optional_payload_text(request, "content_type")
                .unwrap_or_else(|| "application/json".to_owned());
            let output = client(config)
                .create_hosted_configuration_version()
                .application_id(application_id)
                .configuration_profile_id(profile_id)
                .content_type(content_type)
                .content(Blob::new(content.into_bytes()))
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error(
                        "appconfig",
                        "create_hosted_configuration_version",
                        err,
                    )
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!(
                    "Created AppConfig hosted configuration version `{}`.",
                    output.version_number()
                ),
                resource_id: Some(format!(
                    "hosted-version/{application_id}/{profile_id}/{}",
                    output.version_number()
                )),
            })
        }
        "start_deployment" => {
            let version_path = require_resource_id(request, "hosted-version/")?;
            let mut parts = version_path.splitn(3, '/');
            let application_id = parts.next().unwrap_or_default();
            let profile_id = parts.next().unwrap_or_default();
            let version = parts.next().unwrap_or("1");
            let environment_id =
                optional_payload_text(request, "environment_id").ok_or_else(|| {
                    ServiceManagementError::invalid_input(
                        request.service_key.clone(),
                        request.action.clone(),
                        "`environment_id` is required.",
                    )
                })?;
            let strategy_id = optional_payload_text(request, "deployment_strategy_id")
                .unwrap_or_else(|| "AppConfig.AllAtOnce".to_owned());
            let output = client(config)
                .start_deployment()
                .application_id(application_id)
                .environment_id(&environment_id)
                .configuration_profile_id(profile_id)
                .configuration_version(version)
                .deployment_strategy_id(strategy_id)
                .send()
                .await
                .map_err(|err| {
                    ServiceManagementError::client_error("appconfig", "start_deployment", err)
                })?;

            Ok(ActionResult {
                changed: true,
                message: format!(
                    "Started AppConfig deployment #{} for environment `{environment_id}`.",
                    output.deployment_number()
                ),
                resource_id: Some(format!(
                    "deployment/{application_id}/{environment_id}/{}",
                    output.deployment_number()
                )),
            })
        }
        _ => Err(unsupported_action(request)),
    }
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        tab(
            "applications",
            "Applications",
            &["application"],
            "No AppConfig applications were found.",
        ),
        tab(
            "environments",
            "Environments",
            &["environment"],
            "No AppConfig environments were found.",
        ),
        tab(
            "profiles",
            "Profiles",
            &["configuration-profile"],
            "No AppConfig configuration profiles were found.",
        ),
        tab(
            "versions",
            "Versions",
            &["hosted-version"],
            "No hosted configuration versions were found.",
        ),
        tab(
            "deployments",
            "Deployments",
            &["deployment"],
            "No AppConfig deployments were found.",
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

fn client(config: &AppConfig) -> aws_sdk_appconfig::Client {
    let sdk_config = aws_sdk_appconfig::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_appconfig::config::Region::new(
            config.region.clone(),
        ))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_appconfig::Client::from_conf(sdk_config)
}
