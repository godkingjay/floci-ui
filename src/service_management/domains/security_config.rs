use super::{ActionKind, ServiceActionDefinition, ServiceDomainDefinition};

pub fn definition(service_key: &str) -> Option<&'static ServiceDomainDefinition> {
    DEFINITIONS
        .iter()
        .find(|definition| definition.key == service_key)
}

pub fn is_security_config_service(service_key: &str) -> bool {
    definition(service_key).is_some()
}

const IAM_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_user",
        label: "Create user",
        kind: ActionKind::Create,
        resource_kind: Some("user"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "create_role",
        label: "Create role",
        kind: ActionKind::Create,
        resource_kind: Some("role"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "create_policy",
        label: "Create policy",
        kind: ActionKind::Create,
        resource_kind: Some("policy"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "create_access_key",
        label: "Create access key",
        kind: ActionKind::Execute,
        resource_kind: Some("user"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_user",
        label: "Delete user",
        kind: ActionKind::Delete,
        resource_kind: Some("user"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_role",
        label: "Delete role",
        kind: ActionKind::Delete,
        resource_kind: Some("role"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_policy",
        label: "Delete policy",
        kind: ActionKind::Delete,
        resource_kind: Some("policy"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_access_key",
        label: "Delete access key",
        kind: ActionKind::Delete,
        resource_kind: Some("access-key"),
        requires_selection: true,
    },
];

const READ_ONLY_ACTIONS: &[ServiceActionDefinition] = &[ServiceActionDefinition {
    key: "refresh_inventory",
    label: "Refresh metadata",
    kind: ActionKind::Refresh,
    resource_kind: None,
    requires_selection: false,
}];

const COGNITO_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_user_pool",
        label: "Create user pool",
        kind: ActionKind::Create,
        resource_kind: Some("user-pool"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "create_user_pool_client",
        label: "Create app client",
        kind: ActionKind::Execute,
        resource_kind: Some("user-pool"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_user_pool",
        label: "Delete user pool",
        kind: ActionKind::Delete,
        resource_kind: Some("user-pool"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_user_pool_client",
        label: "Delete app client",
        kind: ActionKind::Delete,
        resource_kind: Some("app-client"),
        requires_selection: true,
    },
];

const KMS_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_key",
        label: "Create key",
        kind: ActionKind::Create,
        resource_kind: Some("key"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "create_alias",
        label: "Create alias",
        kind: ActionKind::Execute,
        resource_kind: Some("key"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "refresh_key_policy",
        label: "Refresh policy",
        kind: ActionKind::Refresh,
        resource_kind: Some("key"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "disable_key",
        label: "Disable key",
        kind: ActionKind::Delete,
        resource_kind: Some("key"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "schedule_key_deletion",
        label: "Schedule deletion",
        kind: ActionKind::Delete,
        resource_kind: Some("key"),
        requires_selection: true,
    },
];

const SECRETS_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "reveal_secret_value",
        label: "Reveal value",
        kind: ActionKind::Execute,
        resource_kind: Some("secret"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "rotate_secret",
        label: "Rotate secret",
        kind: ActionKind::Delete,
        resource_kind: Some("secret"),
        requires_selection: true,
    },
];

const SSM_ACTIONS: &[ServiceActionDefinition] = &[ServiceActionDefinition {
    key: "reveal_parameter_value",
    label: "Reveal value",
    kind: ActionKind::Execute,
    resource_kind: Some("parameter"),
    requires_selection: true,
}];

const APPCONFIG_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_application",
        label: "Create application",
        kind: ActionKind::Create,
        resource_kind: Some("application"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "create_environment",
        label: "Create environment",
        kind: ActionKind::Execute,
        resource_kind: Some("application"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "create_configuration_profile",
        label: "Create profile",
        kind: ActionKind::Execute,
        resource_kind: Some("application"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "create_hosted_version",
        label: "Create version",
        kind: ActionKind::Execute,
        resource_kind: Some("configuration-profile"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "start_deployment",
        label: "Start deployment",
        kind: ActionKind::Execute,
        resource_kind: Some("hosted-version"),
        requires_selection: true,
    },
];

const ACM_ACTIONS: &[ServiceActionDefinition] = &[ServiceActionDefinition {
    key: "import_certificate",
    label: "Import disabled",
    kind: ActionKind::Unsupported,
    resource_kind: None,
    requires_selection: false,
}];

const DEFINITIONS: &[ServiceDomainDefinition] = &[
    ServiceDomainDefinition {
        key: "iam",
        domain_label: "Security and configuration",
        summary: "Manage local identity metadata with typed deletes and redacted access-key secrets.",
        create_field_label: Some("Name"),
        create_placeholder: Some("floci-local"),
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: IAM_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "sts",
        domain_label: "Security and configuration",
        summary: "Inspect caller identity and local session context without exposing credentials.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: READ_ONLY_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "cognito",
        domain_label: "Security and configuration",
        summary: "Inspect user pools, app clients, users, groups, and providers.",
        create_field_label: Some("User pool name"),
        create_placeholder: Some("floci-local-users"),
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: COGNITO_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "kms",
        domain_label: "Security and configuration",
        summary: "Inspect keys, aliases, grants, and policies while key material remains unavailable.",
        create_field_label: Some("Description"),
        create_placeholder: Some("floci local key"),
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: KMS_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "secretsmanager",
        domain_label: "Security and configuration",
        summary: "Inspect secret metadata and require an explicit reveal action for local values.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: SECRETS_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "ssm",
        domain_label: "Security and configuration",
        summary: "Inspect parameters, documents, commands, and managed instances with secure values hidden.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: SSM_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "appconfig",
        domain_label: "Security and configuration",
        summary: "Manage local applications, environments, profiles, hosted versions, and deployments.",
        create_field_label: Some("Application name"),
        create_placeholder: Some("floci-local-config"),
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: APPCONFIG_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "appconfigdata",
        domain_label: "Security and configuration",
        summary: "Inspect AppConfigData session and payload-preview metadata without persisting tokens.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: READ_ONLY_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "acm",
        domain_label: "Security and configuration",
        summary: "Inspect certificates, domains, and validation metadata without loading private material.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: ACM_ACTIONS,
    },
];
