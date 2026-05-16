use super::{ActionKind, ServiceActionDefinition, ServiceDomainDefinition};

pub fn definition(service_key: &str) -> Option<&'static ServiceDomainDefinition> {
    DEFINITIONS
        .iter()
        .find(|definition| definition.key == service_key)
}

pub fn is_compute_build_service(service_key: &str) -> bool {
    definition(service_key).is_some()
}

const LAMBDA_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_function",
        label: "Create function",
        kind: ActionKind::Create,
        resource_kind: Some("function"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "invoke_function",
        label: "Invoke function",
        kind: ActionKind::Execute,
        resource_kind: Some("function"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "refresh_function_configuration",
        label: "Refresh configuration",
        kind: ActionKind::Refresh,
        resource_kind: Some("function"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_function",
        label: "Delete function",
        kind: ActionKind::Delete,
        resource_kind: Some("function"),
        requires_selection: true,
    },
];

const EC2_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "start_instance",
        label: "Start instance",
        kind: ActionKind::Execute,
        resource_kind: Some("instance"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "stop_instance",
        label: "Stop instance",
        kind: ActionKind::Delete,
        resource_kind: Some("instance"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "terminate_instance",
        label: "Terminate instance",
        kind: ActionKind::Delete,
        resource_kind: Some("instance"),
        requires_selection: true,
    },
];

const ECS_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "run_task",
        label: "Run task",
        kind: ActionKind::Execute,
        resource_kind: Some("cluster"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "stop_task",
        label: "Stop task",
        kind: ActionKind::Delete,
        resource_kind: Some("task"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "update_service_desired_count",
        label: "Update desired count",
        kind: ActionKind::Execute,
        resource_kind: Some("service"),
        requires_selection: true,
    },
];

const ECR_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_repository",
        label: "Create repository",
        kind: ActionKind::Create,
        resource_kind: Some("repository"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "delete_repository",
        label: "Delete repository",
        kind: ActionKind::Delete,
        resource_kind: Some("repository"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_image",
        label: "Delete image",
        kind: ActionKind::Delete,
        resource_kind: Some("image"),
        requires_selection: true,
    },
];

const CODEBUILD_ACTIONS: &[ServiceActionDefinition] = &[ServiceActionDefinition {
    key: "start_build",
    label: "Start build",
    kind: ActionKind::Execute,
    resource_kind: Some("project"),
    requires_selection: true,
}];

const CODEDEPLOY_ACTIONS: &[ServiceActionDefinition] = &[ServiceActionDefinition {
    key: "create_deployment",
    label: "Create deployment",
    kind: ActionKind::Execute,
    resource_kind: Some("deployment-group"),
    requires_selection: true,
}];

const AUTOSCALING_ACTIONS: &[ServiceActionDefinition] = &[ServiceActionDefinition {
    key: "update_desired_capacity",
    label: "Update desired capacity",
    kind: ActionKind::Execute,
    resource_kind: Some("auto-scaling-group"),
    requires_selection: true,
}];

const BEDROCK_RUNTIME_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "invoke_model",
        label: "Invoke model",
        kind: ActionKind::Execute,
        resource_kind: Some("model"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "validate_request_template",
        label: "Validate request",
        kind: ActionKind::Execute,
        resource_kind: Some("request-template"),
        requires_selection: true,
    },
];

const READ_ONLY_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "refresh_inventory",
        label: "Refresh metadata",
        kind: ActionKind::Refresh,
        resource_kind: None,
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "delete",
        label: "Delete unsupported",
        kind: ActionKind::Unsupported,
        resource_kind: None,
        requires_selection: true,
    },
];

const DEFINITIONS: &[ServiceDomainDefinition] = &[
    ServiceDomainDefinition {
        key: "lambda",
        domain_label: "Compute, container, and build",
        summary: "Inspect local functions, invoke JSON payloads, and keep configuration secrets hidden.",
        create_field_label: Some("Function name"),
        create_placeholder: Some("process-order"),
        secondary_field_label: Some("Role ARN"),
        secondary_placeholder: Some("arn:aws:iam::000000000000:role/floci-lambda-local"),
        actions: LAMBDA_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "ec2",
        domain_label: "Compute, container, and build",
        summary: "Inspect instances and networking state with typed confirmations for stop and terminate.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: EC2_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "ecs",
        domain_label: "Compute, container, and build",
        summary: "Inspect clusters, services, tasks, and definitions with guarded task/service actions.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: ECS_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "eks",
        domain_label: "Compute, container, and build",
        summary: "Inspect EKS cluster, node group, add-on, and endpoint metadata without mutating Kubernetes state.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: READ_ONLY_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "ecr",
        domain_label: "Compute, container, and build",
        summary: "Manage local repositories and image metadata with destructive confirmation.",
        create_field_label: Some("Repository name"),
        create_placeholder: Some("orders-api"),
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: ECR_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "msk",
        domain_label: "Compute, container, and build",
        summary: "Inspect Kafka clusters, brokers, and configurations while mutation remains disabled.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: READ_ONLY_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "codebuild",
        domain_label: "Compute, container, and build",
        summary: "Inspect projects, builds, reports, and start local builds from a selected project.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: CODEBUILD_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "codedeploy",
        domain_label: "Compute, container, and build",
        summary: "Inspect applications and deployment groups, then create local deployment records.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: CODEDEPLOY_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "autoscaling",
        domain_label: "Compute, container, and build",
        summary: "Inspect scaling groups, instances, policies, and update desired capacity with numeric validation.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: AUTOSCALING_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "bedrockruntime",
        domain_label: "Compute, container, and build",
        summary: "Invoke local model endpoints with explicit JSON request preview and summarized responses.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: BEDROCK_RUNTIME_ACTIONS,
    },
];
