use super::{ActionKind, ServiceActionDefinition, ServiceDomainDefinition};

pub fn definition(service_key: &str) -> Option<&'static ServiceDomainDefinition> {
    DEFINITIONS
        .iter()
        .find(|definition| definition.key == service_key)
}

pub fn is_network_observability_service(service_key: &str) -> bool {
    definition(service_key).is_some()
}

const APIGATEWAY_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_rest_api",
        label: "Create REST API",
        kind: ActionKind::Create,
        resource_kind: Some("rest-api"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "create_resource",
        label: "Create resource",
        kind: ActionKind::Execute,
        resource_kind: Some("resource"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "put_method",
        label: "Put method",
        kind: ActionKind::Execute,
        resource_kind: Some("resource"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "put_integration",
        label: "Put integration",
        kind: ActionKind::Execute,
        resource_kind: Some("method"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "create_deployment",
        label: "Create deployment",
        kind: ActionKind::Execute,
        resource_kind: Some("rest-api"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_rest_api",
        label: "Delete REST API",
        kind: ActionKind::Delete,
        resource_kind: Some("rest-api"),
        requires_selection: true,
    },
];

const APIGATEWAYV2_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_api",
        label: "Create API",
        kind: ActionKind::Create,
        resource_kind: Some("api"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "create_route",
        label: "Create route",
        kind: ActionKind::Execute,
        resource_kind: Some("api"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "create_integration",
        label: "Create integration",
        kind: ActionKind::Execute,
        resource_kind: Some("api"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "create_deployment",
        label: "Create deployment",
        kind: ActionKind::Execute,
        resource_kind: Some("api"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_api",
        label: "Delete API",
        kind: ActionKind::Delete,
        resource_kind: Some("api"),
        requires_selection: true,
    },
];

const ELBV2_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_target_group",
        label: "Create target group",
        kind: ActionKind::Create,
        resource_kind: Some("target-group"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "register_target",
        label: "Register target",
        kind: ActionKind::Execute,
        resource_kind: Some("target-group"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "deregister_target",
        label: "Deregister target",
        kind: ActionKind::Delete,
        resource_kind: Some("target"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_listener",
        label: "Delete listener",
        kind: ActionKind::Delete,
        resource_kind: Some("listener"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_load_balancer",
        label: "Delete load balancer",
        kind: ActionKind::Delete,
        resource_kind: Some("load-balancer"),
        requires_selection: true,
    },
];

const ROUTE53_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_hosted_zone",
        label: "Create hosted zone",
        kind: ActionKind::Create,
        resource_kind: Some("hosted-zone"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "upsert_record",
        label: "Upsert record",
        kind: ActionKind::Execute,
        resource_kind: Some("hosted-zone"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_record",
        label: "Delete record",
        kind: ActionKind::Delete,
        resource_kind: Some("record-set"),
        requires_selection: true,
    },
];

const TRANSFER_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_server",
        label: "Create server",
        kind: ActionKind::Create,
        resource_kind: Some("server"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "create_user",
        label: "Create user",
        kind: ActionKind::Execute,
        resource_kind: Some("server"),
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
        key: "delete_server",
        label: "Delete server",
        kind: ActionKind::Delete,
        resource_kind: Some("server"),
        requires_selection: true,
    },
];

const CLOUDWATCHLOGS_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_log_group",
        label: "Create log group",
        kind: ActionKind::Create,
        resource_kind: Some("log-group"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "tail_recent_events",
        label: "Tail events",
        kind: ActionKind::Execute,
        resource_kind: Some("log-group"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_log_group",
        label: "Delete log group",
        kind: ActionKind::Delete,
        resource_kind: Some("log-group"),
        requires_selection: true,
    },
];

const CLOUDWATCH_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "query_metric_data",
        label: "Query metric",
        kind: ActionKind::Execute,
        resource_kind: Some("metric"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "create_alarm",
        label: "Create alarm",
        kind: ActionKind::Execute,
        resource_kind: Some("metric"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_alarm",
        label: "Delete alarm",
        kind: ActionKind::Delete,
        resource_kind: Some("alarm"),
        requires_selection: true,
    },
];

const DEFINITIONS: &[ServiceDomainDefinition] = &[
    ServiceDomainDefinition {
        key: "apigateway",
        domain_label: "Network, observability, and edge",
        summary: "Manage local REST APIs, resources, methods, integrations, deployments, and stages.",
        create_field_label: Some("REST API name"),
        create_placeholder: Some("orders-rest-api"),
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: APIGATEWAY_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "apigatewayv2",
        domain_label: "Network, observability, and edge",
        summary: "Manage local HTTP and WebSocket APIs with routes, integrations, stages, and authorizers.",
        create_field_label: Some("API name"),
        create_placeholder: Some("orders-http-api"),
        secondary_field_label: Some("Protocol"),
        secondary_placeholder: Some("HTTP"),
        actions: APIGATEWAYV2_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "elbv2",
        domain_label: "Network, observability, and edge",
        summary: "Inspect local load balancers, listeners, rules, target groups, and target health.",
        create_field_label: Some("Target group name"),
        create_placeholder: Some("orders-targets"),
        secondary_field_label: Some("VPC ID"),
        secondary_placeholder: Some("vpc-00000000"),
        actions: ELBV2_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "route53",
        domain_label: "Network, observability, and edge",
        summary: "Manage local hosted zones and record sets while health checks remain inspectable.",
        create_field_label: Some("Hosted zone name"),
        create_placeholder: Some("local.floci.test."),
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: ROUTE53_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "transfer",
        domain_label: "Network, observability, and edge",
        summary: "Manage local Transfer Family servers and users while identity material stays redacted.",
        create_field_label: Some("Protocol"),
        create_placeholder: Some("SFTP"),
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: TRANSFER_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "cloudwatchlogs",
        domain_label: "Network, observability, and edge",
        summary: "Manage local log groups and inspect streams, filters, subscriptions, and redacted events.",
        create_field_label: Some("Log group name"),
        create_placeholder: Some("/aws/lambda/orders-worker"),
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: CLOUDWATCHLOGS_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "cloudwatch",
        domain_label: "Network, observability, and edge",
        summary: "Inspect local metrics, alarms, and dashboards with explicit metric-query actions.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: CLOUDWATCH_ACTIONS,
    },
];
