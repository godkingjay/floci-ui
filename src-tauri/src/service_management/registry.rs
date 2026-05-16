use crate::models::{ServiceCategory, ServiceDescriptor};

use super::{
    errors::ServiceManagementError,
    models::{ResourceTab, ServiceInventory, ServiceSupportLevel},
};

#[derive(Debug, Clone, Copy)]
pub struct StaticServiceDescriptor {
    pub key: &'static str,
    pub label: &'static str,
    pub category: ServiceCategory,
    pub description: &'static str,
    pub domain_epic: &'static str,
    pub support_level: ServiceSupportLevel,
    pub primary_resource_kinds: &'static [&'static str],
    pub safe_operations: &'static [&'static str],
}

pub const SERVICE_DESCRIPTORS: &[StaticServiceDescriptor] = &[
    StaticServiceDescriptor {
        key: "s3",
        label: "S3",
        category: ServiceCategory::Data,
        description: "Buckets, objects, policies, and object-lock state.",
        domain_epic: "Data and storage",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &[
            "bucket",
            "object-prefix",
            "bucket-policy",
            "lifecycle-rule",
            "tag-set",
        ],
        safe_operations: &["list", "inspect", "create_bucket", "delete_bucket"],
    },
    StaticServiceDescriptor {
        key: "dynamodb",
        label: "DynamoDB",
        category: ServiceCategory::Data,
        description: "Tables, streams, item counts, indexes, and TTL state.",
        domain_epic: "Data and storage",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &[
            "table",
            "global-secondary-index",
            "local-secondary-index",
            "stream",
            "ttl",
            "backup",
        ],
        safe_operations: &["list", "inspect", "create_table", "delete_table"],
    },
    StaticServiceDescriptor {
        key: "dynamodbstreams",
        label: "DynamoDB Streams",
        category: ServiceCategory::Data,
        description: "Table stream shards, sequence ranges, and stream status.",
        domain_epic: "Data and storage",
        support_level: ServiceSupportLevel::ReadOnly,
        primary_resource_kinds: &["stream", "shard"],
        safe_operations: &["list", "inspect"],
    },
    StaticServiceDescriptor {
        key: "rds",
        label: "RDS",
        category: ServiceCategory::Data,
        description: "Instances, clusters, subnet groups, and snapshots.",
        domain_epic: "Data and storage",
        support_level: ServiceSupportLevel::ReadOnly,
        primary_resource_kinds: &[
            "db-instance",
            "db-cluster",
            "snapshot",
            "subnet-group",
            "parameter-group",
        ],
        safe_operations: &["list", "inspect"],
    },
    StaticServiceDescriptor {
        key: "elasticache",
        label: "ElastiCache",
        category: ServiceCategory::Data,
        description: "Redis/Valkey clusters, users, and auth settings.",
        domain_epic: "Data and storage",
        support_level: ServiceSupportLevel::ReadOnly,
        primary_resource_kinds: &["cache-cluster", "replication-group", "user", "subnet-group"],
        safe_operations: &["list", "inspect"],
    },
    StaticServiceDescriptor {
        key: "opensearch",
        label: "OpenSearch",
        category: ServiceCategory::Data,
        description: "Domains, endpoints, and backing container state.",
        domain_epic: "Data and storage",
        support_level: ServiceSupportLevel::ReadOnly,
        primary_resource_kinds: &["domain", "endpoint", "index-hint", "access-policy"],
        safe_operations: &["list", "inspect"],
    },
    StaticServiceDescriptor {
        key: "glue",
        label: "Glue",
        category: ServiceCategory::Data,
        description: "Catalog databases, tables, crawlers, jobs, and connections.",
        domain_epic: "Data and storage",
        support_level: ServiceSupportLevel::ReadOnly,
        primary_resource_kinds: &["database", "table", "connection", "job", "crawler"],
        safe_operations: &["list", "inspect"],
    },
    StaticServiceDescriptor {
        key: "athena",
        label: "Athena",
        category: ServiceCategory::Data,
        description: "Workgroups, data catalogs, named queries, and result metadata.",
        domain_epic: "Data and storage",
        support_level: ServiceSupportLevel::ReadOnly,
        primary_resource_kinds: &[
            "workgroup",
            "named-query",
            "result-metadata",
            "data-catalog",
        ],
        safe_operations: &["list", "inspect"],
    },
    StaticServiceDescriptor {
        key: "firehose",
        label: "Data Firehose",
        category: ServiceCategory::Data,
        description: "Delivery streams, destinations, and buffering state.",
        domain_epic: "Data and storage",
        support_level: ServiceSupportLevel::ReadOnly,
        primary_resource_kinds: &["delivery-stream", "destination"],
        safe_operations: &["list", "inspect"],
    },
    StaticServiceDescriptor {
        key: "backup",
        label: "AWS Backup",
        category: ServiceCategory::Data,
        description: "Backup vaults, plans, jobs, and recovery points.",
        domain_epic: "Data and storage",
        support_level: ServiceSupportLevel::ReadOnly,
        primary_resource_kinds: &[
            "backup-vault",
            "backup-plan",
            "backup-job",
            "recovery-point",
        ],
        safe_operations: &["list", "inspect"],
    },
    StaticServiceDescriptor {
        key: "sqs",
        label: "SQS",
        category: ServiceCategory::Messaging,
        description: "Queues, attributes, redrive policy, and queue depth.",
        domain_epic: "Messaging, events, and workflows",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &["queue", "dead-letter-queue", "queue-attribute", "message"],
        safe_operations: &[
            "list",
            "inspect",
            "create_queue",
            "send_message",
            "purge_queue",
            "delete_queue",
        ],
    },
    StaticServiceDescriptor {
        key: "sns",
        label: "SNS",
        category: ServiceCategory::Messaging,
        description: "Topics, subscriptions, and delivery policies.",
        domain_epic: "Messaging, events, and workflows",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &["topic", "subscription", "policy", "delivery-policy"],
        safe_operations: &[
            "list",
            "inspect",
            "create_topic",
            "publish_message",
            "subscribe_endpoint",
            "delete_topic",
            "delete_subscription",
        ],
    },
    StaticServiceDescriptor {
        key: "ses",
        label: "SES",
        category: ServiceCategory::Messaging,
        description: "Identities, templates, suppression lists, and send state.",
        domain_epic: "Messaging, events, and workflows",
        support_level: ServiceSupportLevel::ReadOnly,
        primary_resource_kinds: &[
            "identity",
            "template",
            "configuration-set",
            "suppression",
            "send-statistic",
        ],
        safe_operations: &["list", "inspect"],
    },
    StaticServiceDescriptor {
        key: "sesv2",
        label: "SES v2",
        category: ServiceCategory::Messaging,
        description: "Email identities, configuration sets, and contact lists.",
        domain_epic: "Messaging, events, and workflows",
        support_level: ServiceSupportLevel::ReadOnly,
        primary_resource_kinds: &[
            "email-identity",
            "template",
            "configuration-set",
            "contact-list",
            "account-setting",
            "suppression",
        ],
        safe_operations: &["list", "inspect"],
    },
    StaticServiceDescriptor {
        key: "kinesis",
        label: "Kinesis",
        category: ServiceCategory::Messaging,
        description: "Streams, shards, consumers, and retention settings.",
        domain_epic: "Messaging, events, and workflows",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &["stream", "shard", "consumer", "metric"],
        safe_operations: &[
            "list",
            "inspect",
            "create_stream",
            "put_record",
            "delete_stream",
        ],
    },
    StaticServiceDescriptor {
        key: "eventbridge",
        label: "EventBridge",
        category: ServiceCategory::Messaging,
        description: "Event buses, rules, targets, and archives.",
        domain_epic: "Messaging, events, and workflows",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &["event-bus", "rule", "target", "archive", "replay"],
        safe_operations: &[
            "list",
            "inspect",
            "create_event_bus",
            "put_event",
            "create_rule",
            "delete_event_bus",
            "delete_rule",
        ],
    },
    StaticServiceDescriptor {
        key: "scheduler",
        label: "EventBridge Scheduler",
        category: ServiceCategory::Messaging,
        description: "Schedules, schedule groups, targets, and flexible windows.",
        domain_epic: "Messaging, events, and workflows",
        support_level: ServiceSupportLevel::ReadOnly,
        primary_resource_kinds: &["schedule", "schedule-group", "target"],
        safe_operations: &["list", "inspect"],
    },
    StaticServiceDescriptor {
        key: "stepfunctions",
        label: "Step Functions",
        category: ServiceCategory::Messaging,
        description: "State machines, executions, activities, and aliases.",
        domain_epic: "Messaging, events, and workflows",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &[
            "state-machine",
            "execution",
            "activity",
            "definition",
            "alias",
        ],
        safe_operations: &[
            "list",
            "inspect",
            "create_state_machine",
            "start_execution",
            "stop_execution",
            "delete_state_machine",
        ],
    },
    StaticServiceDescriptor {
        key: "cloudformation",
        label: "CloudFormation",
        category: ServiceCategory::Messaging,
        description: "Stacks, change sets, templates, and stack events.",
        domain_epic: "Messaging, events, and workflows",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &[
            "stack",
            "stack-resource",
            "stack-event",
            "change-set",
            "parameter",
        ],
        safe_operations: &[
            "list",
            "inspect",
            "create_stack",
            "view_stack_events",
            "delete_stack",
        ],
    },
    StaticServiceDescriptor {
        key: "lambda",
        label: "Lambda",
        category: ServiceCategory::Compute,
        description: "Functions, versions, aliases, event sources, and runtime metadata.",
        domain_epic: "Compute, container, and build",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &[
            "function",
            "version",
            "alias",
            "event-source-mapping",
            "configuration",
        ],
        safe_operations: &[
            "list",
            "inspect",
            "create_function",
            "invoke_function",
            "delete_function",
        ],
    },
    StaticServiceDescriptor {
        key: "ec2",
        label: "EC2",
        category: ServiceCategory::Compute,
        description: "Instances, VPCs, security groups, key pairs, subnets, and volumes.",
        domain_epic: "Compute, container, and build",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &[
            "instance",
            "vpc",
            "subnet",
            "security-group",
            "key-pair",
            "image",
            "volume",
        ],
        safe_operations: &["list", "inspect", "start_instance", "stop_instance"],
    },
    StaticServiceDescriptor {
        key: "ecs",
        label: "ECS",
        category: ServiceCategory::Compute,
        description: "Clusters, services, tasks, and task definitions.",
        domain_epic: "Compute, container, and build",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &[
            "cluster",
            "service",
            "task",
            "task-definition",
            "container-instance",
        ],
        safe_operations: &[
            "list",
            "inspect",
            "run_task",
            "stop_task",
            "update_service_desired_count",
        ],
    },
    StaticServiceDescriptor {
        key: "eks",
        label: "EKS",
        category: ServiceCategory::Compute,
        description: "Clusters, node groups, add-ons, and Kubernetes endpoint metadata.",
        domain_epic: "Compute, container, and build",
        support_level: ServiceSupportLevel::ReadOnly,
        primary_resource_kinds: &["cluster", "node-group", "addon"],
        safe_operations: &["list", "inspect"],
    },
    StaticServiceDescriptor {
        key: "ecr",
        label: "ECR",
        category: ServiceCategory::Compute,
        description: "Repositories, images, lifecycle policies, and scan findings.",
        domain_epic: "Compute, container, and build",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &["repository", "image", "tag"],
        safe_operations: &[
            "list",
            "inspect",
            "create_repository",
            "delete_repository",
            "delete_image",
        ],
    },
    StaticServiceDescriptor {
        key: "msk",
        label: "MSK",
        category: ServiceCategory::Compute,
        description: "Kafka clusters, configurations, brokers, and topic hints.",
        domain_epic: "Compute, container, and build",
        support_level: ServiceSupportLevel::ReadOnly,
        primary_resource_kinds: &["cluster", "broker", "configuration"],
        safe_operations: &["list", "inspect"],
    },
    StaticServiceDescriptor {
        key: "codebuild",
        label: "CodeBuild",
        category: ServiceCategory::Compute,
        description: "Projects, builds, build batches, and report groups.",
        domain_epic: "Compute, container, and build",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &["project", "build", "report"],
        safe_operations: &["list", "inspect", "start_build"],
    },
    StaticServiceDescriptor {
        key: "codedeploy",
        label: "CodeDeploy",
        category: ServiceCategory::Compute,
        description: "Applications, deployments, deployment groups, and revisions.",
        domain_epic: "Compute, container, and build",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &["application", "deployment-group", "deployment"],
        safe_operations: &["list", "inspect", "create_deployment"],
    },
    StaticServiceDescriptor {
        key: "autoscaling",
        label: "Auto Scaling",
        category: ServiceCategory::Compute,
        description: "Auto Scaling groups, launch configurations, and scaling policies.",
        domain_epic: "Compute, container, and build",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &[
            "auto-scaling-group",
            "scaling-instance",
            "launch-configuration",
            "scaling-policy",
            "lifecycle-hook",
        ],
        safe_operations: &["list", "inspect", "update_desired_capacity"],
    },
    StaticServiceDescriptor {
        key: "bedrockruntime",
        label: "Bedrock Runtime",
        category: ServiceCategory::Compute,
        description: "Model invocations, jobs, and local runtime request traces.",
        domain_epic: "Compute, container, and build",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &[
            "model",
            "invocation",
            "request-template",
            "response-preview",
        ],
        safe_operations: &["list", "inspect", "invoke_model"],
    },
    StaticServiceDescriptor {
        key: "iam",
        label: "IAM",
        category: ServiceCategory::Security,
        description: "Users, roles, groups, policies, and access keys.",
        domain_epic: "Security and configuration",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &[
            "user",
            "role",
            "group",
            "policy",
            "access-key",
            "instance-profile",
        ],
        safe_operations: &[
            "list",
            "inspect",
            "create_user",
            "delete_user",
            "create_role",
            "delete_role",
            "create_policy",
            "delete_policy",
            "create_access_key",
            "delete_access_key",
        ],
    },
    StaticServiceDescriptor {
        key: "sts",
        label: "STS",
        category: ServiceCategory::Security,
        description: "Caller identity, sessions, and token metadata.",
        domain_epic: "Security and configuration",
        support_level: ServiceSupportLevel::ReadOnly,
        primary_resource_kinds: &["caller-identity", "session"],
        safe_operations: &["list", "inspect"],
    },
    StaticServiceDescriptor {
        key: "cognito",
        label: "Cognito",
        category: ServiceCategory::Security,
        description: "User pools, app clients, identity providers, and groups.",
        domain_epic: "Security and configuration",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &[
            "user-pool",
            "app-client",
            "user",
            "group",
            "identity-provider",
        ],
        safe_operations: &[
            "list",
            "inspect",
            "create_user_pool",
            "delete_user_pool",
            "create_user_pool_client",
            "delete_user_pool_client",
        ],
    },
    StaticServiceDescriptor {
        key: "kms",
        label: "KMS",
        category: ServiceCategory::Security,
        description: "Keys, aliases, grants, and key policy metadata.",
        domain_epic: "Security and configuration",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &["key", "alias", "grant", "key-policy"],
        safe_operations: &[
            "list",
            "inspect",
            "create_key",
            "create_alias",
            "disable_key",
            "schedule_key_deletion",
        ],
    },
    StaticServiceDescriptor {
        key: "secretsmanager",
        label: "Secrets Manager",
        category: ServiceCategory::Security,
        description: "Secrets, versions, rotation settings, and tags.",
        domain_epic: "Security and configuration",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &["secret", "secret-version", "rotation", "tag-set"],
        safe_operations: &["list", "inspect", "reveal_secret_value", "rotate_secret"],
    },
    StaticServiceDescriptor {
        key: "ssm",
        label: "SSM",
        category: ServiceCategory::Core,
        description: "Parameters, run command state, documents, and managed instances.",
        domain_epic: "Security and configuration",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &["parameter", "command", "managed-instance", "document"],
        safe_operations: &["list", "inspect", "reveal_parameter_value"],
    },
    StaticServiceDescriptor {
        key: "appconfig",
        label: "AppConfig",
        category: ServiceCategory::Core,
        description: "Applications, environments, configuration profiles, and deployments.",
        domain_epic: "Security and configuration",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &[
            "application",
            "environment",
            "configuration-profile",
            "hosted-version",
            "deployment",
        ],
        safe_operations: &[
            "list",
            "inspect",
            "create_application",
            "create_environment",
            "create_configuration_profile",
            "create_hosted_version",
            "start_deployment",
        ],
    },
    StaticServiceDescriptor {
        key: "appconfigdata",
        label: "AppConfig Data",
        category: ServiceCategory::Core,
        description: "Configuration sessions, tokens, and fetched configuration payload metadata.",
        domain_epic: "Security and configuration",
        support_level: ServiceSupportLevel::ReadOnly,
        primary_resource_kinds: &["session-preview", "configuration-preview"],
        safe_operations: &["list", "inspect"],
    },
    StaticServiceDescriptor {
        key: "acm",
        label: "ACM",
        category: ServiceCategory::Security,
        description: "Certificates, domain validation records, and renewal metadata.",
        domain_epic: "Security and configuration",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &["certificate", "domain", "validation", "tag-set"],
        safe_operations: &["list", "inspect"],
    },
    StaticServiceDescriptor {
        key: "apigateway",
        label: "API Gateway",
        category: ServiceCategory::Networking,
        description: "REST APIs, resources, methods, integrations, and stages.",
        domain_epic: "Network, observability, and edge",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &[
            "rest-api",
            "resource",
            "method",
            "integration",
            "deployment",
            "stage",
        ],
        safe_operations: &[
            "list",
            "inspect",
            "create_rest_api",
            "create_resource",
            "put_method",
            "put_integration",
            "create_deployment",
            "delete_rest_api",
        ],
    },
    StaticServiceDescriptor {
        key: "apigatewayv2",
        label: "API Gateway v2",
        category: ServiceCategory::Networking,
        description: "HTTP/WebSocket APIs, routes, integrations, deployments, and stages.",
        domain_epic: "Network, observability, and edge",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &[
            "api",
            "route",
            "integration",
            "deployment",
            "stage",
            "authorizer",
        ],
        safe_operations: &[
            "list",
            "inspect",
            "create_api",
            "create_route",
            "create_integration",
            "create_deployment",
            "delete_api",
        ],
    },
    StaticServiceDescriptor {
        key: "elbv2",
        label: "Elastic Load Balancing v2",
        category: ServiceCategory::Networking,
        description: "Load balancers, listeners, target groups, and target health.",
        domain_epic: "Network, observability, and edge",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &[
            "load-balancer",
            "listener",
            "listener-rule",
            "target-group",
            "target",
        ],
        safe_operations: &[
            "list",
            "inspect",
            "create_target_group",
            "register_target",
            "deregister_target",
            "delete_listener",
            "delete_load_balancer",
        ],
    },
    StaticServiceDescriptor {
        key: "route53",
        label: "Route 53",
        category: ServiceCategory::Networking,
        description: "Hosted zones, record sets, reusable delegation sets, and health checks.",
        domain_epic: "Network, observability, and edge",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &["hosted-zone", "record-set", "health-check", "change-batch"],
        safe_operations: &[
            "list",
            "inspect",
            "create_hosted_zone",
            "upsert_record",
            "delete_record",
        ],
    },
    StaticServiceDescriptor {
        key: "transfer",
        label: "Transfer Family",
        category: ServiceCategory::Networking,
        description: "Servers, users, workflows, host keys, and identity provider state.",
        domain_epic: "Network, observability, and edge",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &["server", "user", "workflow", "host-key"],
        safe_operations: &[
            "list",
            "inspect",
            "create_server",
            "create_user",
            "delete_user",
            "delete_server",
        ],
    },
    StaticServiceDescriptor {
        key: "cloudwatchlogs",
        label: "CloudWatch Logs",
        category: ServiceCategory::Observability,
        description: "Log groups, log streams, metric filters, and query metadata.",
        domain_epic: "Network, observability, and edge",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &[
            "log-group",
            "log-stream",
            "log-event",
            "metric-filter",
            "subscription-filter",
        ],
        safe_operations: &[
            "list",
            "inspect",
            "create_log_group",
            "tail_recent_events",
            "delete_log_group",
        ],
    },
    StaticServiceDescriptor {
        key: "cloudwatch",
        label: "CloudWatch",
        category: ServiceCategory::Observability,
        description: "Metrics, alarms, dashboards, and alarm history.",
        domain_epic: "Network, observability, and edge",
        support_level: ServiceSupportLevel::Managed,
        primary_resource_kinds: &["metric", "alarm", "dashboard", "query-result"],
        safe_operations: &[
            "list",
            "inspect",
            "query_metric_data",
            "create_alarm",
            "delete_alarm",
        ],
    },
];

pub fn services() -> Vec<ServiceDescriptor> {
    SERVICE_DESCRIPTORS
        .iter()
        .map(|service| ServiceDescriptor {
            key: service.key.to_owned(),
            label: service.label.to_owned(),
            category: service.category,
            description: service.description.to_owned(),
            domain_epic: service.domain_epic.to_owned(),
            support_level: service.support_level,
            primary_resource_kinds: service
                .primary_resource_kinds
                .iter()
                .map(|kind| (*kind).to_owned())
                .collect(),
            safe_operations: service
                .safe_operations
                .iter()
                .map(|operation| (*operation).to_owned())
                .collect(),
        })
        .collect()
}

pub fn descriptor(service_key: &str) -> Option<&'static StaticServiceDescriptor> {
    SERVICE_DESCRIPTORS
        .iter()
        .find(|service| service.key == service_key)
}

pub fn unsupported_inventory(
    service_key: &str,
) -> Result<ServiceInventory, ServiceManagementError> {
    let descriptor = descriptor(service_key)
        .ok_or_else(|| ServiceManagementError::unsupported_service(service_key))?;
    let tabs = descriptor
        .primary_resource_kinds
        .iter()
        .map(|kind| ResourceTab {
            key: (*kind).to_owned(),
            label: title_case_kind(kind),
            kinds: vec![(*kind).to_owned()],
            empty_message: format!(
                "{} `{kind}` inventory is not available until an adapter is implemented.",
                descriptor.label
            ),
        })
        .collect();

    Ok(ServiceInventory {
        service_key: descriptor.key.to_owned(),
        service_label: descriptor.label.to_owned(),
        support_level: descriptor.support_level,
        refreshed_at: super::actions::now_rfc3339(),
        tabs,
        resources: Vec::new(),
        unsupported_operations: vec![
            "list".to_owned(),
            "inspect".to_owned(),
            "create".to_owned(),
            "update".to_owned(),
            "delete".to_owned(),
        ],
    })
}

pub fn title_case_kind(kind: &str) -> String {
    kind.split('-')
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::SERVICE_DESCRIPTORS;

    #[test]
    fn registry_contains_45_unique_services() {
        let keys = SERVICE_DESCRIPTORS
            .iter()
            .map(|service| service.key)
            .collect::<std::collections::BTreeSet<_>>();

        assert_eq!(SERVICE_DESCRIPTORS.len(), 45);
        assert_eq!(keys.len(), 45);
    }

    #[test]
    fn registry_has_no_duplicate_labels_within_category() {
        let mut seen = std::collections::BTreeSet::new();

        for service in SERVICE_DESCRIPTORS {
            let inserted = seen.insert((service.category as u8, service.label));
            assert!(
                inserted,
                "duplicate label `{}` in category {:?}",
                service.label, service.category
            );
        }
    }
}
