use std::collections::BTreeMap;

use crate::{
    config::AppConfig,
    service_management::{
        adapters::security_config::{
            format_timestamp, insert_attr, local_credentials, managed_inventory, unsupported_action,
        },
        errors::ServiceManagementError,
        models::{
            ActionResult, ResourceSummary, ResourceTab, ServiceActionRequest, ServiceInventory,
        },
    },
};

#[allow(clippy::too_many_lines)]
pub async fn list_resources(
    config: &AppConfig,
) -> Result<ServiceInventory, ServiceManagementError> {
    let client = client(config);
    let certificates = client
        .list_certificates()
        .send()
        .await
        .map_err(|err| ServiceManagementError::client_error("acm", "list_certificates", err))?;
    let mut resources = Vec::new();

    for certificate in certificates.certificate_summary_list() {
        let Some(arn) = certificate.certificate_arn() else {
            continue;
        };
        let domain_name = certificate.domain_name().unwrap_or(arn);
        let mut attributes = BTreeMap::new();
        attributes.insert("certificate_arn".to_owned(), arn.to_owned());
        attributes.insert("private_key_material".to_owned(), "not loaded".to_owned());
        insert_attr(
            &mut attributes,
            "type",
            certificate.r#type().map(ToString::to_string),
        );
        insert_attr(
            &mut attributes,
            "key_algorithm",
            certificate.key_algorithm().map(ToString::to_string),
        );
        insert_attr(&mut attributes, "in_use", certificate.in_use());
        insert_attr(
            &mut attributes,
            "renewal_eligibility",
            certificate.renewal_eligibility().map(ToString::to_string),
        );
        let sans = certificate.subject_alternative_name_summaries().join(", ");
        if !sans.is_empty() {
            attributes.insert("subject_alternative_names".to_owned(), sans);
        }

        let mut status = certificate
            .status()
            .map_or_else(|| "unknown".to_owned(), ToString::to_string);
        let mut created_at = format_timestamp(certificate.created_at());
        let mut updated_at = format_timestamp(certificate.issued_at());

        if let Ok(description) = client
            .describe_certificate()
            .certificate_arn(arn)
            .send()
            .await
        {
            if let Some(detail) = description.certificate() {
                status = detail.status().map(ToString::to_string).unwrap_or(status);
                created_at = format_timestamp(detail.created_at());
                updated_at =
                    format_timestamp(detail.issued_at()).or(format_timestamp(detail.imported_at()));
                insert_attr(&mut attributes, "subject", detail.subject());
                insert_attr(&mut attributes, "issuer", detail.issuer());
                insert_attr(&mut attributes, "serial", detail.serial());
                insert_attr(
                    &mut attributes,
                    "signature_algorithm",
                    detail.signature_algorithm(),
                );
                insert_attr(
                    &mut attributes,
                    "not_before",
                    format_timestamp(detail.not_before()),
                );
                insert_attr(
                    &mut attributes,
                    "not_after",
                    format_timestamp(detail.not_after()),
                );

                for validation in detail.domain_validation_options() {
                    let domain = validation.domain_name();
                    let mut validation_attributes = BTreeMap::from([
                        ("certificate_arn".to_owned(), arn.to_owned()),
                        ("domain_name".to_owned(), domain.to_owned()),
                    ]);
                    insert_attr(
                        &mut validation_attributes,
                        "validation_domain",
                        validation.validation_domain(),
                    );
                    insert_attr(
                        &mut validation_attributes,
                        "validation_method",
                        validation.validation_method().map(ToString::to_string),
                    );
                    if let Some(record) = validation.resource_record() {
                        validation_attributes
                            .insert("record_name".to_owned(), record.name().to_owned());
                        validation_attributes
                            .insert("record_type".to_owned(), record.r#type().to_string());
                        validation_attributes
                            .insert("record_value".to_owned(), record.value().to_owned());
                    }

                    resources.push(ResourceSummary {
                        id: format!("validation/{arn}/{domain}"),
                        name: domain.to_owned(),
                        kind: "validation".to_owned(),
                        status: validation
                            .validation_status()
                            .map_or_else(|| "unknown".to_owned(), ToString::to_string),
                        created_at: None,
                        updated_at: None,
                        tags: BTreeMap::new(),
                        attributes: validation_attributes,
                    });
                }
            }
        }

        resources.push(ResourceSummary {
            id: format!("certificate/{arn}"),
            name: domain_name.to_owned(),
            kind: "certificate".to_owned(),
            status,
            created_at,
            updated_at,
            tags: BTreeMap::new(),
            attributes,
        });

        resources.push(ResourceSummary {
            id: format!("domain/{arn}/{domain_name}"),
            name: domain_name.to_owned(),
            kind: "domain".to_owned(),
            status: "covered".to_owned(),
            created_at: None,
            updated_at: None,
            tags: BTreeMap::new(),
            attributes: BTreeMap::from([("certificate_arn".to_owned(), arn.to_owned())]),
        });
    }

    Ok(managed_inventory(
        "acm",
        "ACM",
        tabs(),
        resources,
        vec![
            "import_certificate".to_owned(),
            "delete_certificate".to_owned(),
            "export_certificate".to_owned(),
        ],
    ))
}

pub async fn execute_action(
    _config: &AppConfig,
    request: &ServiceActionRequest,
) -> Result<ActionResult, ServiceManagementError> {
    if request.action == "import_certificate" {
        return Err(ServiceManagementError::unsupported_operation(
            "acm",
            "import_certificate",
            "ACM certificate import is disabled because the current action contract would place private key material in frontend state.",
        ));
    }

    Err(unsupported_action(request))
}

pub fn tabs() -> Vec<ResourceTab> {
    vec![
        tab(
            "certificates",
            "Certificates",
            &["certificate"],
            "No ACM certificates were found.",
        ),
        tab(
            "domains",
            "Domains",
            &["domain"],
            "No ACM certificate domains were found.",
        ),
        tab(
            "validation",
            "Validation",
            &["validation"],
            "No ACM validation metadata was found.",
        ),
        tab(
            "tags",
            "Tags",
            &["tag-set"],
            "ACM tags are folded into resource metadata.",
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

fn client(config: &AppConfig) -> aws_sdk_acm::Client {
    let sdk_config = aws_sdk_acm::Config::builder()
        .behavior_version_latest()
        .endpoint_url(config.endpoint_url.to_string())
        .region(aws_sdk_acm::config::Region::new(config.region.clone()))
        .credentials_provider(local_credentials(config))
        .build();

    aws_sdk_acm::Client::from_conf(sdk_config)
}
