use js_sys::{Function, Promise, Reflect};
use serde::{Serialize, de::DeserializeOwned};
#[cfg(all(target_arch = "wasm32", debug_assertions))]
use serde_json::json;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

/// Invoke a Tauri command and deserialize its JavaScript response into `T`.
///
/// # Errors
///
/// Returns an error when the Tauri bridge is unavailable, the command rejects,
/// or the response cannot be deserialized into the requested type.
pub async fn invoke_command<T>(command: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    invoke_command_internal(command, JsValue::UNDEFINED, None).await
}

/// Invoke a Tauri command with JSON-serializable arguments.
///
/// # Errors
///
/// Returns an error when arguments cannot be serialized, the Tauri bridge is
/// unavailable, the command rejects, or the response cannot be deserialized.
pub async fn invoke_command_with_args<T, A>(command: &str, args: &A) -> Result<T, String>
where
    T: DeserializeOwned,
    A: Serialize,
{
    let preview_args = serde_json::to_value(args).map_err(|err| err.to_string())?;
    let args = serde_wasm_bindgen::to_value(args).map_err(|err| err.to_string())?;

    invoke_command_internal(command, args, Some(preview_args)).await
}

async fn invoke_command_internal<T>(
    command: &str,
    args: JsValue,
    preview_args: Option<serde_json::Value>,
) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let (this_arg, invoke) = match tauri_invoke() {
        Ok(command_bridge) => command_bridge,
        Err(message) => {
            if let Some(preview_result) = browser_preview_result(command, preview_args.as_ref()) {
                return preview_result;
            }

            return Err(message);
        }
    };
    let promise = invoke
        .call2(&this_arg, &JsValue::from_str(command), &args)
        .map_err(|value| js_error(&value))?;
    let promise = promise
        .dyn_into::<Promise>()
        .map_err(|_| "Tauri invoke did not return a Promise.".to_owned())?;
    let value = JsFuture::from(promise)
        .await
        .map_err(|value| js_error(&value))?;

    serde_wasm_bindgen::from_value(value).map_err(|err| err.to_string())
}

fn tauri_invoke() -> Result<(JsValue, Function), String> {
    let window = web_sys::window().ok_or_else(|| "No browser window is available.".to_owned())?;
    let tauri = Reflect::get(window.as_ref(), &JsValue::from_str("__TAURI__"))
        .map_err(|value| js_error(&value))?;

    if tauri.is_null() || tauri.is_undefined() {
        return Err("Tauri APIs are unavailable. Start the app with `cargo tauri dev`.".to_owned());
    }

    let core =
        Reflect::get(&tauri, &JsValue::from_str("core")).map_err(|value| js_error(&value))?;
    let invoke =
        Reflect::get(&core, &JsValue::from_str("invoke")).map_err(|value| js_error(&value))?;
    let invoke = invoke
        .dyn_into::<Function>()
        .map_err(|_| "window.__TAURI__.core.invoke is not a function.".to_owned())?;

    Ok((core, invoke))
}

#[cfg(all(target_arch = "wasm32", debug_assertions))]
fn browser_preview_result<T>(
    command: &str,
    args: Option<&serde_json::Value>,
) -> Option<Result<T, String>>
where
    T: DeserializeOwned,
{
    if !is_local_browser_preview() {
        return None;
    }

    let value = match command {
        "floci_health" => json!({
            "ok": true,
            "url": "http://localhost:4566/_localstack/health",
            "status": 200,
            "floci_version": "preview",
            "health_status": "running",
            "body": {
                "status": "running",
                "version": "preview",
                "services": {
                    "s3": "running",
                    "dynamodb": "running",
                    "sqs": "available",
                    "lambda": "available"
                },
                "edition": "local-preview"
            },
            "error": null
        }),
        "service_catalog" => browser_preview_service_catalog(),
        "service_inventory" => browser_preview_inventory(args)?,
        "service_resource_detail" => browser_preview_resource_detail(args)?,
        "service_execute_action" => browser_preview_action(args)?,
        _ => return None,
    };

    Some(serde_json::from_value(value).map_err(|err| err.to_string()))
}

#[cfg(not(all(target_arch = "wasm32", debug_assertions)))]
fn browser_preview_result<T>(
    _command: &str,
    _args: Option<&serde_json::Value>,
) -> Option<Result<T, String>>
where
    T: DeserializeOwned,
{
    None
}

#[cfg(all(target_arch = "wasm32", debug_assertions))]
#[derive(Clone, Copy)]
struct PreviewServiceDescriptor {
    key: &'static str,
    label: &'static str,
    category: &'static str,
    description: &'static str,
    domain_epic: &'static str,
    support_level: &'static str,
    primary_resource_kinds: &'static [&'static str],
    safe_operations: &'static [&'static str],
}

#[cfg(all(target_arch = "wasm32", debug_assertions))]
const PREVIEW_SERVICE_DESCRIPTORS: &[PreviewServiceDescriptor] = &[
    preview_service(
        "s3",
        "S3",
        "data",
        "Buckets, objects, policies, and object-lock state.",
        "Data and storage",
        "managed",
        &[
            "bucket",
            "object-prefix",
            "bucket-policy",
            "lifecycle-rule",
            "tag-set",
        ],
        &["list", "inspect", "create_bucket", "delete_bucket"],
    ),
    preview_service(
        "dynamodb",
        "DynamoDB",
        "data",
        "Tables, streams, item counts, indexes, and TTL state.",
        "Data and storage",
        "managed",
        &[
            "table",
            "global-secondary-index",
            "local-secondary-index",
            "stream",
            "ttl",
            "backup",
        ],
        &["list", "inspect", "create_table", "delete_table"],
    ),
    preview_service(
        "dynamodbstreams",
        "DynamoDB Streams",
        "data",
        "Table stream shards, sequence ranges, and stream status.",
        "Data and storage",
        "read-only",
        &["stream", "shard"],
        &["list", "inspect"],
    ),
    preview_service(
        "rds",
        "RDS",
        "data",
        "Instances, clusters, subnet groups, and snapshots.",
        "Data and storage",
        "read-only",
        &[
            "db-instance",
            "db-cluster",
            "snapshot",
            "subnet-group",
            "parameter-group",
        ],
        &["list", "inspect"],
    ),
    preview_service(
        "elasticache",
        "ElastiCache",
        "data",
        "Redis/Valkey clusters, users, and auth settings.",
        "Data and storage",
        "read-only",
        &["cache-cluster", "replication-group", "user", "subnet-group"],
        &["list", "inspect"],
    ),
    preview_service(
        "opensearch",
        "OpenSearch",
        "data",
        "Domains, endpoints, and backing container state.",
        "Data and storage",
        "read-only",
        &["domain", "endpoint", "index-hint", "access-policy"],
        &["list", "inspect"],
    ),
    preview_service(
        "glue",
        "Glue",
        "data",
        "Catalog databases, tables, crawlers, jobs, and connections.",
        "Data and storage",
        "read-only",
        &["database", "table", "connection", "job", "crawler"],
        &["list", "inspect"],
    ),
    preview_service(
        "athena",
        "Athena",
        "data",
        "Workgroups, data catalogs, named queries, and result metadata.",
        "Data and storage",
        "read-only",
        &[
            "workgroup",
            "named-query",
            "result-metadata",
            "data-catalog",
        ],
        &["list", "inspect"],
    ),
    preview_service(
        "firehose",
        "Data Firehose",
        "data",
        "Delivery streams, destinations, and buffering state.",
        "Data and storage",
        "read-only",
        &["delivery-stream", "destination"],
        &["list", "inspect"],
    ),
    preview_service(
        "backup",
        "AWS Backup",
        "data",
        "Backup vaults, plans, jobs, and recovery points.",
        "Data and storage",
        "read-only",
        &[
            "backup-vault",
            "backup-plan",
            "backup-job",
            "recovery-point",
        ],
        &["list", "inspect"],
    ),
    preview_service(
        "sqs",
        "SQS",
        "messaging",
        "Queues, attributes, redrive policy, and queue depth.",
        "Messaging, events, and workflows",
        "managed",
        &["queue", "dead-letter-queue", "queue-attribute", "message"],
        &[
            "list",
            "inspect",
            "create_queue",
            "send_message",
            "purge_queue",
            "delete_queue",
        ],
    ),
    preview_service(
        "sns",
        "SNS",
        "messaging",
        "Topics, subscriptions, and delivery policies.",
        "Messaging, events, and workflows",
        "managed",
        &["topic", "subscription", "policy", "delivery-policy"],
        &[
            "list",
            "inspect",
            "create_topic",
            "publish_message",
            "subscribe_endpoint",
            "delete_topic",
            "delete_subscription",
        ],
    ),
    preview_service(
        "ses",
        "SES",
        "messaging",
        "Identities, templates, suppression lists, and send state.",
        "Messaging, events, and workflows",
        "read-only",
        &[
            "identity",
            "template",
            "configuration-set",
            "suppression",
            "send-statistic",
        ],
        &["list", "inspect"],
    ),
    preview_service(
        "sesv2",
        "SES v2",
        "messaging",
        "Email identities, configuration sets, and contact lists.",
        "Messaging, events, and workflows",
        "read-only",
        &[
            "email-identity",
            "template",
            "configuration-set",
            "contact-list",
            "account-setting",
            "suppression",
        ],
        &["list", "inspect"],
    ),
    preview_service(
        "kinesis",
        "Kinesis",
        "messaging",
        "Streams, shards, consumers, and retention settings.",
        "Messaging, events, and workflows",
        "managed",
        &["stream", "shard", "consumer", "metric"],
        &[
            "list",
            "inspect",
            "create_stream",
            "put_record",
            "delete_stream",
        ],
    ),
    preview_service(
        "eventbridge",
        "EventBridge",
        "messaging",
        "Event buses, rules, targets, and archives.",
        "Messaging, events, and workflows",
        "managed",
        &["event-bus", "rule", "target", "archive", "replay"],
        &[
            "list",
            "inspect",
            "create_event_bus",
            "put_event",
            "create_rule",
            "delete_event_bus",
            "delete_rule",
        ],
    ),
    preview_service(
        "scheduler",
        "EventBridge Scheduler",
        "messaging",
        "Schedules, schedule groups, targets, and flexible windows.",
        "Messaging, events, and workflows",
        "read-only",
        &["schedule", "schedule-group", "target"],
        &["list", "inspect"],
    ),
    preview_service(
        "stepfunctions",
        "Step Functions",
        "messaging",
        "State machines, executions, activities, and aliases.",
        "Messaging, events, and workflows",
        "managed",
        &[
            "state-machine",
            "execution",
            "activity",
            "definition",
            "alias",
        ],
        &[
            "list",
            "inspect",
            "create_state_machine",
            "start_execution",
            "stop_execution",
            "delete_state_machine",
        ],
    ),
    preview_service(
        "cloudformation",
        "CloudFormation",
        "messaging",
        "Stacks, change sets, templates, and stack events.",
        "Messaging, events, and workflows",
        "managed",
        &[
            "stack",
            "stack-resource",
            "stack-event",
            "change-set",
            "parameter",
        ],
        &[
            "list",
            "inspect",
            "create_stack",
            "view_stack_events",
            "delete_stack",
        ],
    ),
    preview_service(
        "lambda",
        "Lambda",
        "compute",
        "Functions, versions, aliases, event sources, and runtime metadata.",
        "Compute, container, and build",
        "managed",
        &[
            "function",
            "version",
            "alias",
            "event-source-mapping",
            "configuration",
        ],
        &[
            "list",
            "inspect",
            "create_function",
            "invoke_function",
            "delete_function",
        ],
    ),
    preview_service(
        "ec2",
        "EC2",
        "compute",
        "Instances, VPCs, security groups, key pairs, subnets, and volumes.",
        "Compute, container, and build",
        "managed",
        &[
            "instance",
            "vpc",
            "subnet",
            "security-group",
            "key-pair",
            "image",
            "volume",
        ],
        &["list", "inspect", "start_instance", "stop_instance"],
    ),
    preview_service(
        "ecs",
        "ECS",
        "compute",
        "Clusters, services, tasks, and task definitions.",
        "Compute, container, and build",
        "managed",
        &[
            "cluster",
            "service",
            "task",
            "task-definition",
            "container-instance",
        ],
        &[
            "list",
            "inspect",
            "run_task",
            "stop_task",
            "update_service_desired_count",
        ],
    ),
    preview_service(
        "eks",
        "EKS",
        "compute",
        "Clusters, node groups, add-ons, and Kubernetes endpoint metadata.",
        "Compute, container, and build",
        "read-only",
        &["cluster", "node-group", "addon"],
        &["list", "inspect"],
    ),
    preview_service(
        "ecr",
        "ECR",
        "compute",
        "Repositories, images, lifecycle policies, and scan findings.",
        "Compute, container, and build",
        "managed",
        &["repository", "image", "tag"],
        &[
            "list",
            "inspect",
            "create_repository",
            "delete_repository",
            "delete_image",
        ],
    ),
    preview_service(
        "msk",
        "MSK",
        "compute",
        "Kafka clusters, configurations, brokers, and topic hints.",
        "Compute, container, and build",
        "read-only",
        &["cluster", "broker", "configuration"],
        &["list", "inspect"],
    ),
    preview_service(
        "codebuild",
        "CodeBuild",
        "compute",
        "Projects, builds, build batches, and report groups.",
        "Compute, container, and build",
        "managed",
        &["project", "build", "report"],
        &["list", "inspect", "start_build"],
    ),
    preview_service(
        "codedeploy",
        "CodeDeploy",
        "compute",
        "Applications, deployments, deployment groups, and revisions.",
        "Compute, container, and build",
        "managed",
        &["application", "deployment-group", "deployment"],
        &["list", "inspect", "create_deployment"],
    ),
    preview_service(
        "autoscaling",
        "Auto Scaling",
        "compute",
        "Auto Scaling groups, launch configurations, and scaling policies.",
        "Compute, container, and build",
        "managed",
        &[
            "auto-scaling-group",
            "scaling-instance",
            "launch-configuration",
            "scaling-policy",
            "lifecycle-hook",
        ],
        &["list", "inspect", "update_desired_capacity"],
    ),
    preview_service(
        "bedrockruntime",
        "Bedrock Runtime",
        "compute",
        "Model invocations, jobs, and local runtime request traces.",
        "Compute, container, and build",
        "managed",
        &[
            "model",
            "invocation",
            "request-template",
            "response-preview",
        ],
        &["list", "inspect", "invoke_model"],
    ),
    preview_service(
        "iam",
        "IAM",
        "security",
        "Users, roles, groups, policies, and access keys.",
        "Security and configuration",
        "managed",
        &[
            "user",
            "role",
            "group",
            "policy",
            "access-key",
            "instance-profile",
        ],
        &[
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
    ),
    preview_service(
        "sts",
        "STS",
        "security",
        "Caller identity, sessions, and token metadata.",
        "Security and configuration",
        "read-only",
        &["caller-identity", "session"],
        &["list", "inspect"],
    ),
    preview_service(
        "cognito",
        "Cognito",
        "security",
        "User pools, app clients, identity providers, and groups.",
        "Security and configuration",
        "managed",
        &[
            "user-pool",
            "app-client",
            "user",
            "group",
            "identity-provider",
        ],
        &[
            "list",
            "inspect",
            "create_user_pool",
            "delete_user_pool",
            "create_user_pool_client",
            "delete_user_pool_client",
        ],
    ),
    preview_service(
        "kms",
        "KMS",
        "security",
        "Keys, aliases, grants, and key policy metadata.",
        "Security and configuration",
        "managed",
        &["key", "alias", "grant", "key-policy"],
        &[
            "list",
            "inspect",
            "create_key",
            "create_alias",
            "disable_key",
            "schedule_key_deletion",
        ],
    ),
    preview_service(
        "secretsmanager",
        "Secrets Manager",
        "security",
        "Secrets, versions, rotation settings, and tags.",
        "Security and configuration",
        "managed",
        &["secret", "secret-version", "rotation", "tag-set"],
        &["list", "inspect", "reveal_secret_value", "rotate_secret"],
    ),
    preview_service(
        "ssm",
        "SSM",
        "core",
        "Parameters, run command state, documents, and managed instances.",
        "Security and configuration",
        "managed",
        &["parameter", "command", "managed-instance", "document"],
        &["list", "inspect", "reveal_parameter_value"],
    ),
    preview_service(
        "appconfig",
        "AppConfig",
        "core",
        "Applications, environments, configuration profiles, and deployments.",
        "Security and configuration",
        "managed",
        &[
            "application",
            "environment",
            "configuration-profile",
            "hosted-version",
            "deployment",
        ],
        &[
            "list",
            "inspect",
            "create_application",
            "create_environment",
            "create_configuration_profile",
            "create_hosted_version",
            "start_deployment",
        ],
    ),
    preview_service(
        "appconfigdata",
        "AppConfig Data",
        "core",
        "Configuration sessions, tokens, and fetched configuration payload metadata.",
        "Security and configuration",
        "read-only",
        &["session-preview", "configuration-preview"],
        &["list", "inspect"],
    ),
    preview_service(
        "acm",
        "ACM",
        "security",
        "Certificates, domain validation records, and renewal metadata.",
        "Security and configuration",
        "managed",
        &["certificate", "domain", "validation", "tag-set"],
        &["list", "inspect"],
    ),
    preview_service(
        "apigateway",
        "API Gateway",
        "networking",
        "REST APIs, resources, methods, integrations, and stages.",
        "Network, observability, and edge",
        "managed",
        &[
            "rest-api",
            "resource",
            "method",
            "integration",
            "deployment",
            "stage",
        ],
        &[
            "list",
            "inspect",
            "create_rest_api",
            "create_resource",
            "put_method",
            "put_integration",
            "create_deployment",
            "delete_rest_api",
        ],
    ),
    preview_service(
        "apigatewayv2",
        "API Gateway v2",
        "networking",
        "HTTP/WebSocket APIs, routes, integrations, deployments, and stages.",
        "Network, observability, and edge",
        "managed",
        &[
            "api",
            "route",
            "integration",
            "deployment",
            "stage",
            "authorizer",
        ],
        &[
            "list",
            "inspect",
            "create_api",
            "create_route",
            "create_integration",
            "create_deployment",
            "delete_api",
        ],
    ),
    preview_service(
        "elbv2",
        "Elastic Load Balancing v2",
        "networking",
        "Load balancers, listeners, target groups, and target health.",
        "Network, observability, and edge",
        "managed",
        &[
            "load-balancer",
            "listener",
            "listener-rule",
            "target-group",
            "target",
        ],
        &[
            "list",
            "inspect",
            "create_target_group",
            "register_target",
            "deregister_target",
            "delete_listener",
            "delete_load_balancer",
        ],
    ),
    preview_service(
        "route53",
        "Route 53",
        "networking",
        "Hosted zones, record sets, reusable delegation sets, and health checks.",
        "Network, observability, and edge",
        "managed",
        &["hosted-zone", "record-set", "health-check", "change-batch"],
        &[
            "list",
            "inspect",
            "create_hosted_zone",
            "upsert_record",
            "delete_record",
        ],
    ),
    preview_service(
        "transfer",
        "Transfer Family",
        "networking",
        "Servers, users, workflows, host keys, and identity provider state.",
        "Network, observability, and edge",
        "managed",
        &["server", "user", "workflow", "host-key"],
        &[
            "list",
            "inspect",
            "create_server",
            "create_user",
            "delete_user",
            "delete_server",
        ],
    ),
    preview_service(
        "cloudwatchlogs",
        "CloudWatch Logs",
        "observability",
        "Log groups, log streams, metric filters, and query metadata.",
        "Network, observability, and edge",
        "managed",
        &[
            "log-group",
            "log-stream",
            "log-event",
            "metric-filter",
            "subscription-filter",
        ],
        &[
            "list",
            "inspect",
            "create_log_group",
            "tail_recent_events",
            "delete_log_group",
        ],
    ),
    preview_service(
        "cloudwatch",
        "CloudWatch",
        "observability",
        "Metrics, alarms, dashboards, and alarm history.",
        "Network, observability, and edge",
        "managed",
        &["metric", "alarm", "dashboard", "query-result"],
        &[
            "list",
            "inspect",
            "query_metric_data",
            "create_alarm",
            "delete_alarm",
        ],
    ),
];

#[cfg(all(target_arch = "wasm32", debug_assertions))]
const fn preview_service(
    key: &'static str,
    label: &'static str,
    category: &'static str,
    description: &'static str,
    domain_epic: &'static str,
    support_level: &'static str,
    primary_resource_kinds: &'static [&'static str],
    safe_operations: &'static [&'static str],
) -> PreviewServiceDescriptor {
    PreviewServiceDescriptor {
        key,
        label,
        category,
        description,
        domain_epic,
        support_level,
        primary_resource_kinds,
        safe_operations,
    }
}

#[cfg(all(target_arch = "wasm32", debug_assertions))]
fn browser_preview_service_catalog() -> serde_json::Value {
    json!({
        "endpoint_url": "http://localhost:4566",
        "region": "us-east-1",
        "access_key_id": "test",
        "credentials_status": "Configured",
        "last_refreshed_at": browser_preview_timestamp(),
        "services": PREVIEW_SERVICE_DESCRIPTORS
            .iter()
            .map(preview_service_json)
            .collect::<Vec<_>>()
    })
}

#[cfg(all(target_arch = "wasm32", debug_assertions))]
fn preview_service_json(service: &PreviewServiceDescriptor) -> serde_json::Value {
    json!({
        "key": service.key,
        "label": service.label,
        "category": service.category,
        "description": service.description,
        "domain_epic": service.domain_epic,
        "support_level": service.support_level,
        "primary_resource_kinds": service.primary_resource_kinds,
        "safe_operations": service.safe_operations
    })
}

#[cfg(all(target_arch = "wasm32", debug_assertions))]
fn preview_service_descriptor(service_key: &str) -> Option<&'static PreviewServiceDescriptor> {
    PREVIEW_SERVICE_DESCRIPTORS
        .iter()
        .find(|service| service.key == service_key)
}

#[cfg(all(target_arch = "wasm32", debug_assertions))]
fn browser_preview_inventory(args: Option<&serde_json::Value>) -> Option<serde_json::Value> {
    let service_key = preview_request_value(args, "service_key")?;
    let (label, support_level, tabs, resources, unsupported_operations) = match service_key {
        "s3" => (
            "S3",
            "managed",
            json!([
                tab(
                    "buckets",
                    "Buckets",
                    ["bucket"],
                    "No S3 buckets were found."
                ),
                tab(
                    "object-prefixes",
                    "Object Prefixes",
                    ["object-prefix"],
                    "Object prefix discovery is metadata-only for now."
                ),
                tab(
                    "policies",
                    "Policies",
                    ["bucket-policy"],
                    "No bucket policies are loaded."
                ),
                tab(
                    "lifecycle",
                    "Lifecycle",
                    ["lifecycle-rule"],
                    "No lifecycle rules are loaded."
                ),
                tab("tags", "Tags", ["tag-set"], "No S3 tag sets are loaded.")
            ]),
            json!([
                resource(
                    "bucket/assets-local",
                    "assets-local",
                    "bucket",
                    "available",
                    json!({"objects": "18", "region": "us-east-1"})
                ),
                resource(
                    "bucket/floci-logs",
                    "floci-logs",
                    "bucket",
                    "available",
                    json!({"objects": "7", "preview": "metadata-only"})
                )
            ]),
            json!(["empty_bucket", "object_body_preview"]),
        ),
        "dynamodb" => (
            "DynamoDB",
            "managed",
            json!([
                tab(
                    "tables",
                    "Tables",
                    ["table"],
                    "No DynamoDB tables were found."
                ),
                tab(
                    "indexes",
                    "Indexes",
                    ["global-secondary-index", "local-secondary-index"],
                    "No secondary indexes are attached."
                ),
                tab(
                    "streams",
                    "Streams",
                    ["stream"],
                    "No DynamoDB streams are enabled."
                ),
                tab("ttl", "TTL", ["ttl"], "No TTL metadata is loaded."),
                tab(
                    "backups",
                    "Backups",
                    ["backup"],
                    "No DynamoDB backups are loaded."
                )
            ]),
            json!([
                resource(
                    "table/orders",
                    "orders",
                    "table",
                    "active",
                    json!({"items": "42", "billing": "PAY_PER_REQUEST"})
                ),
                resource(
                    "gsi/orders/status-index",
                    "status-index",
                    "global-secondary-index",
                    "active",
                    json!({"table": "orders"})
                ),
                resource(
                    "table/sessions",
                    "sessions",
                    "table",
                    "active",
                    json!({"items": "12", "ttl": "disabled"})
                )
            ]),
            json!(["toggle_ttl"]),
        ),
        "dynamodbstreams" => read_only_preview(
            "DynamoDB Streams",
            json!([tab(
                "streams",
                "Streams",
                ["stream"],
                "No DynamoDB streams are exposed."
            )]),
            json!([resource(
                "stream/orders/latest",
                "orders/latest",
                "stream",
                "enabled",
                json!({"table": "orders"})
            )]),
        ),
        "rds" => read_only_preview(
            "RDS",
            json!([
                tab(
                    "instances",
                    "Instances",
                    ["db-instance"],
                    "No RDS DB instances were found."
                ),
                tab(
                    "clusters",
                    "Clusters",
                    ["db-cluster"],
                    "No RDS DB clusters were found."
                ),
                tab(
                    "snapshots",
                    "Snapshots",
                    ["snapshot"],
                    "No RDS snapshots were found."
                ),
                tab(
                    "subnet-groups",
                    "Subnet Groups",
                    ["subnet-group"],
                    "No RDS subnet groups were found."
                ),
                tab(
                    "parameter-groups",
                    "Parameter Groups",
                    ["parameter-group"],
                    "No RDS parameter groups were found."
                )
            ]),
            json!([
                resource(
                    "db-instance/local-postgres",
                    "local-postgres",
                    "db-instance",
                    "available",
                    json!({"engine": "postgres"})
                ),
                resource(
                    "snapshot/local-postgres/bootstrap",
                    "bootstrap",
                    "snapshot",
                    "available",
                    json!({"source": "local-postgres"})
                )
            ]),
        ),
        "elasticache" => read_only_preview(
            "ElastiCache",
            json!([
                tab(
                    "clusters",
                    "Clusters",
                    ["cache-cluster"],
                    "No ElastiCache clusters were found."
                ),
                tab(
                    "replication-groups",
                    "Replication Groups",
                    ["replication-group"],
                    "No replication groups were found."
                ),
                tab("users", "Users", ["user"], "No users were found."),
                tab(
                    "subnet-groups",
                    "Subnet Groups",
                    ["subnet-group"],
                    "No subnet groups were found."
                )
            ]),
            json!([resource(
                "cache-cluster/session-cache",
                "session-cache",
                "cache-cluster",
                "available",
                json!({"engine": "redis"})
            )]),
        ),
        "opensearch" => read_only_preview(
            "OpenSearch",
            json!([
                tab(
                    "domains",
                    "Domains",
                    ["domain"],
                    "No OpenSearch domains were found."
                ),
                tab(
                    "endpoints",
                    "Endpoints",
                    ["endpoint"],
                    "No endpoints are loaded."
                ),
                tab(
                    "index-hints",
                    "Index Hints",
                    ["index-hint"],
                    "No index hints are loaded."
                ),
                tab(
                    "access-policies",
                    "Access Policies",
                    ["access-policy"],
                    "No access policies are loaded."
                )
            ]),
            json!([resource(
                "domain/search-local",
                "search-local",
                "domain",
                "processing",
                json!({"endpoint": "localhost"})
            )]),
        ),
        "glue" => read_only_preview(
            "Glue",
            json!([
                tab(
                    "catalog",
                    "Catalog",
                    ["database", "table", "connection"],
                    "No Glue catalog entries were found."
                ),
                tab("jobs", "Jobs", ["job"], "No Glue jobs were found."),
                tab(
                    "crawlers",
                    "Crawlers",
                    ["crawler"],
                    "No Glue crawlers were found."
                )
            ]),
            json!([
                resource(
                    "database/local_catalog",
                    "local_catalog",
                    "database",
                    "ready",
                    json!({"tables": "3"})
                ),
                resource(
                    "crawler/orders-crawler",
                    "orders-crawler",
                    "crawler",
                    "stopped",
                    json!({"target": "s3://assets-local/orders"})
                )
            ]),
        ),
        "athena" => read_only_preview(
            "Athena",
            json!([
                tab(
                    "workgroups",
                    "Workgroups",
                    ["workgroup"],
                    "No Athena workgroups were found."
                ),
                tab(
                    "queries",
                    "Queries",
                    ["named-query"],
                    "No named queries were found."
                ),
                tab(
                    "results-metadata",
                    "Results Metadata",
                    ["result-metadata"],
                    "No result metadata is loaded."
                ),
                tab(
                    "catalog",
                    "Catalog",
                    ["data-catalog"],
                    "No data catalogs were found."
                )
            ]),
            json!([resource(
                "workgroup/primary",
                "primary",
                "workgroup",
                "enabled",
                json!({"output": "s3://floci-logs/athena"})
            )]),
        ),
        "firehose" => read_only_preview(
            "Data Firehose",
            json!([
                tab(
                    "streams",
                    "Streams",
                    ["delivery-stream"],
                    "No delivery streams were found."
                ),
                tab(
                    "destinations",
                    "Destinations",
                    ["destination"],
                    "No destinations are loaded."
                )
            ]),
            json!([resource(
                "delivery-stream/audit-stream",
                "audit-stream",
                "delivery-stream",
                "active",
                json!({"destination": "s3"})
            )]),
        ),
        "backup" => read_only_preview(
            "AWS Backup",
            json!([
                tab(
                    "vaults",
                    "Vaults",
                    ["backup-vault"],
                    "No backup vaults were found."
                ),
                tab(
                    "plans",
                    "Plans",
                    ["backup-plan"],
                    "No backup plans were found."
                ),
                tab("jobs", "Jobs", ["backup-job"], "No backup jobs were found."),
                tab(
                    "recovery-points",
                    "Recovery Points",
                    ["recovery-point"],
                    "No recovery points were found."
                )
            ]),
            json!([resource(
                "backup-vault/default",
                "default",
                "backup-vault",
                "available",
                json!({"recovery_points": "0"})
            )]),
        ),
        "sqs" => (
            "SQS",
            "managed",
            json!([
                tab("queues", "Queues", ["queue"], "No SQS queues were found."),
                tab(
                    "dead-letter-queues",
                    "Dead Letter Queues",
                    ["dead-letter-queue"],
                    "No SQS dead-letter queue policies are loaded."
                ),
                tab(
                    "attributes",
                    "Attributes",
                    ["queue-attribute"],
                    "No SQS queue attributes are loaded."
                ),
                tab(
                    "messages",
                    "Messages Preview",
                    ["message"],
                    "Message previews are disabled by default."
                )
            ]),
            json!([
                resource(
                    "queue/http://localhost:4566/000000000000/orders-events",
                    "orders-events",
                    "queue",
                    "available",
                    json!({"ApproximateNumberOfMessages": "4", "VisibilityTimeout": "30"})
                ),
                resource(
                    "queue/http://localhost:4566/000000000000/audit-events",
                    "audit-events",
                    "queue",
                    "available",
                    json!({"ApproximateNumberOfMessages": "0", "MessageRetentionPeriod": "345600"})
                ),
                resource(
                    "queue-attribute/orders-events/VisibilityTimeout",
                    "VisibilityTimeout",
                    "queue-attribute",
                    "loaded",
                    json!({"queue_name": "orders-events", "value": "30"})
                ),
                resource(
                    "dead-letter-queue/orders-events",
                    "orders-events DLQ policy",
                    "dead-letter-queue",
                    "configured",
                    json!({"queue_name": "orders-events", "redrive_policy": "{\"maxReceiveCount\":\"3\"}"})
                )
            ]),
            json!(["message_preview"]),
        ),
        "sns" => (
            "SNS",
            "managed",
            json!([
                tab("topics", "Topics", ["topic"], "No SNS topics were found."),
                tab(
                    "subscriptions",
                    "Subscriptions",
                    ["subscription"],
                    "No SNS subscriptions are loaded."
                ),
                tab(
                    "policies",
                    "Policies",
                    ["policy"],
                    "No SNS policies are loaded."
                ),
                tab(
                    "delivery",
                    "Delivery",
                    ["delivery-policy"],
                    "No SNS delivery policies are loaded."
                )
            ]),
            json!([
                resource(
                    "topic/arn:aws:sns:us-east-1:000000000000:order-updates",
                    "order-updates",
                    "topic",
                    "available",
                    json!({"subscriptions": "2", "owner": "000000000000"})
                ),
                resource(
                    "subscription/arn:aws:sns:us-east-1:000000000000:order-updates:queue",
                    "queue",
                    "subscription",
                    "subscribed",
                    json!({"protocol": "sqs", "endpoint": "orders-events"})
                ),
                resource(
                    "policy/arn:aws:sns:us-east-1:000000000000:order-updates",
                    "order-updates policy",
                    "policy",
                    "attached",
                    json!({"topic_arn": "arn:aws:sns:us-east-1:000000000000:order-updates"})
                ),
                resource(
                    "delivery-policy/arn:aws:sns:us-east-1:000000000000:order-updates",
                    "order-updates delivery",
                    "delivery-policy",
                    "configured",
                    json!({"healthy_retry_policy": "default"})
                )
            ]),
            json!([]),
        ),
        "ses" => read_only_preview(
            "SES",
            json!([
                tab(
                    "identities",
                    "Identities",
                    ["identity"],
                    "No SES identities were found."
                ),
                tab(
                    "templates",
                    "Templates",
                    ["template"],
                    "No SES templates are loaded."
                ),
                tab(
                    "configuration-sets",
                    "Configuration Sets",
                    ["configuration-set"],
                    "No SES configuration sets are loaded."
                ),
                tab(
                    "suppression",
                    "Suppression",
                    ["suppression"],
                    "No SES suppression entries are loaded."
                ),
                tab(
                    "send-statistics",
                    "Send Statistics",
                    ["send-statistic"],
                    "No SES send statistics are loaded."
                )
            ]),
            json!([
                resource(
                    "identity/dev@example.test",
                    "dev@example.test",
                    "identity",
                    "listed",
                    json!({"type": "email"})
                ),
                resource(
                    "template/welcome",
                    "welcome",
                    "template",
                    "available",
                    json!({"subject": "Welcome"})
                ),
                resource(
                    "configuration-set/default",
                    "default",
                    "configuration-set",
                    "available",
                    json!({"sending": "enabled"})
                ),
                resource(
                    "send-statistic/0",
                    "latest send data",
                    "send-statistic",
                    "available",
                    json!({"delivery_attempts": "12", "bounces": "0", "complaints": "0"})
                )
            ]),
        ),
        "sesv2" => read_only_preview(
            "SES v2",
            json!([
                tab(
                    "email-identities",
                    "Identities",
                    ["email-identity"],
                    "No SES v2 identities were found."
                ),
                tab(
                    "templates",
                    "Templates",
                    ["template"],
                    "No SES v2 templates are loaded."
                ),
                tab(
                    "configuration-sets",
                    "Configuration Sets",
                    ["configuration-set"],
                    "No SES v2 configuration sets are loaded."
                ),
                tab(
                    "contact-lists",
                    "Contact Lists",
                    ["contact-list"],
                    "No SES v2 contact lists are loaded."
                ),
                tab(
                    "account-settings",
                    "Account Settings",
                    ["account-setting"],
                    "SES v2 account settings are not available."
                ),
                tab(
                    "suppression",
                    "Suppression",
                    ["suppression"],
                    "No SES v2 suppression entries are loaded."
                )
            ]),
            json!([
                resource(
                    "email-identity/dev@example.test",
                    "dev@example.test",
                    "email-identity",
                    "success",
                    json!({"dkim": "configured"})
                ),
                resource(
                    "configuration-set/default",
                    "default",
                    "configuration-set",
                    "available",
                    json!({"sending": "enabled"})
                ),
                resource(
                    "template/welcome-v2",
                    "welcome-v2",
                    "template",
                    "available",
                    json!({"created": "preview"})
                ),
                resource(
                    "account/default",
                    "Account settings",
                    "account-setting",
                    "sending-enabled",
                    json!({"max_send_rate": "14", "sent_last24_hours": "12"})
                )
            ]),
        ),
        "kinesis" => (
            "Kinesis",
            "managed",
            json!([
                tab(
                    "streams",
                    "Streams",
                    ["stream"],
                    "No Kinesis streams were found."
                ),
                tab(
                    "shards",
                    "Shards",
                    ["shard"],
                    "Shard discovery is summarized at stream level for now."
                ),
                tab(
                    "consumers",
                    "Consumers",
                    ["consumer"],
                    "No consumers are loaded."
                ),
                tab(
                    "metrics",
                    "Metrics",
                    ["metric"],
                    "No Kinesis metrics are loaded."
                )
            ]),
            json!([
                resource(
                    "stream/activity-stream",
                    "activity-stream",
                    "stream",
                    "active",
                    json!({"open_shards": "1", "retention_hours": "24", "stream_mode": "PROVISIONED"})
                ),
                resource(
                    "shard/activity-stream/shardId-000000000000",
                    "shardId-000000000000",
                    "shard",
                    "open",
                    json!({"stream_name": "activity-stream"})
                ),
                resource(
                    "metric/activity-stream",
                    "activity-stream metrics",
                    "metric",
                    "available",
                    json!({"open_shards": "1", "retention_hours": "24"})
                )
            ]),
            json!(["register_consumer", "delete_consumer"]),
        ),
        "eventbridge" => (
            "EventBridge",
            "managed",
            json!([
                tab(
                    "event-buses",
                    "Buses",
                    ["event-bus"],
                    "No event buses were found."
                ),
                tab("rules", "Rules", ["rule"], "No rules are loaded."),
                tab("targets", "Targets", ["target"], "No targets are loaded."),
                tab(
                    "archives",
                    "Archives",
                    ["archive"],
                    "No archives are loaded."
                ),
                tab("replays", "Replays", ["replay"], "No replays are loaded.")
            ]),
            json!([
                resource(
                    "event-bus/app-events",
                    "app-events",
                    "event-bus",
                    "available",
                    json!({"arn": "arn:aws:events:us-east-1:000000000000:event-bus/app-events"})
                ),
                resource(
                    "rule/app-events/order-created",
                    "order-created",
                    "rule",
                    "enabled",
                    json!({"event_bus_name": "app-events", "event_pattern": "{\"source\":[\"orders\"]}"})
                ),
                resource(
                    "target/app-events/order-created/orders-queue",
                    "orders-queue",
                    "target",
                    "attached",
                    json!({"event_bus_name": "app-events", "rule_name": "order-created", "target_arn": "arn:aws:sqs:us-east-1:000000000000:orders-events"})
                ),
                resource(
                    "archive/app-events-archive",
                    "app-events-archive",
                    "archive",
                    "enabled",
                    json!({"event_count": "4", "retention_days": "7"})
                ),
                resource(
                    "replay/manual-replay",
                    "manual-replay",
                    "replay",
                    "completed",
                    json!({"event_source_arn": "arn:aws:events:us-east-1:000000000000:archive/app-events-archive"})
                )
            ]),
            json!(["put_target", "delete_target"]),
        ),
        "scheduler" => read_only_preview(
            "EventBridge Scheduler",
            json!([
                tab(
                    "schedules",
                    "Schedules",
                    ["schedule"],
                    "No schedules were found."
                ),
                tab(
                    "schedule-groups",
                    "Groups",
                    ["schedule-group"],
                    "No schedule groups are loaded."
                ),
                tab(
                    "targets",
                    "Targets",
                    ["target"],
                    "Schedule targets are summarized on schedules."
                )
            ]),
            json!([
                resource(
                    "schedule/default/reconcile-orders",
                    "reconcile-orders",
                    "schedule",
                    "enabled",
                    json!({"group": "default", "expression": "rate(5 minutes)"})
                ),
                resource(
                    "schedule-group/default",
                    "default",
                    "schedule-group",
                    "active",
                    json!({"schedules": "1"})
                ),
                resource(
                    "target/default/reconcile-orders",
                    "arn:aws:sqs:us-east-1:000000000000:orders-events",
                    "target",
                    "attached",
                    json!({"schedule_group": "default", "schedule_name": "reconcile-orders"})
                )
            ]),
        ),
        "stepfunctions" => (
            "Step Functions",
            "managed",
            json!([
                tab(
                    "state-machines",
                    "State Machines",
                    ["state-machine"],
                    "No state machines were found."
                ),
                tab(
                    "executions",
                    "Executions",
                    ["execution"],
                    "No executions are loaded."
                ),
                tab(
                    "activities",
                    "Activities",
                    ["activity"],
                    "No activities are loaded."
                ),
                tab(
                    "definitions",
                    "Definitions",
                    ["definition"],
                    "No definitions are loaded."
                ),
                tab("aliases", "Aliases", ["alias"], "No aliases are loaded.")
            ]),
            json!([
                resource(
                    "state-machine/arn:aws:states:us-east-1:000000000000:stateMachine:order-workflow",
                    "order-workflow",
                    "state-machine",
                    "available",
                    json!({"state_machine_type": "STANDARD"})
                ),
                resource(
                    "execution/arn:aws:states:us-east-1:000000000000:execution:order-workflow:manual-test",
                    "manual-test",
                    "execution",
                    "running",
                    json!({"state_machine_arn": "arn:aws:states:us-east-1:000000000000:stateMachine:order-workflow"})
                ),
                resource(
                    "definition/arn:aws:states:us-east-1:000000000000:stateMachine:order-workflow",
                    "order-workflow definition",
                    "definition",
                    "available",
                    json!({"definition": "{\"StartAt\":\"Pass\",\"States\":{\"Pass\":{\"Type\":\"Pass\",\"End\":true}}}"})
                ),
                resource(
                    "activity/arn:aws:states:us-east-1:000000000000:activity:manual-task",
                    "manual-task",
                    "activity",
                    "available",
                    json!({"activity_arn": "arn:aws:states:us-east-1:000000000000:activity:manual-task"})
                )
            ]),
            json!(["delete_activity"]),
        ),
        "cloudformation" => (
            "CloudFormation",
            "managed",
            json!([
                tab("stacks", "Stacks", ["stack"], "No stacks were found."),
                tab(
                    "resources",
                    "Resources",
                    ["stack-resource"],
                    "No stack resources are loaded."
                ),
                tab(
                    "events",
                    "Events",
                    ["stack-event"],
                    "No stack events are loaded."
                ),
                tab(
                    "change-sets",
                    "Change Sets",
                    ["change-set"],
                    "No change sets are loaded."
                ),
                tab(
                    "parameters",
                    "Parameters",
                    ["parameter"],
                    "No stack parameters are loaded."
                )
            ]),
            json!([
                resource(
                    "stack/bootstrap-stack",
                    "bootstrap-stack",
                    "stack",
                    "create-complete",
                    json!({"resources": "0", "template": "empty"})
                ),
                resource(
                    "stack-event/bootstrap-stack/create-complete",
                    "bootstrap-stack",
                    "stack-event",
                    "create-complete",
                    json!({"resource_type": "AWS::CloudFormation::Stack"})
                ),
                resource(
                    "stack-resource/bootstrap-stack/Root",
                    "Root",
                    "stack-resource",
                    "create-complete",
                    json!({"resource_type": "AWS::CloudFormation::Stack"})
                ),
                resource(
                    "parameter/bootstrap-stack/Environment",
                    "Environment",
                    "parameter",
                    "configured",
                    json!({"value": "local"})
                )
            ]),
            json!([
                "create_change_set",
                "execute_change_set",
                "delete_change_set",
                "update_stack"
            ]),
        ),
        "lambda" => (
            "Lambda",
            "managed",
            json!([
                tab(
                    "functions",
                    "Functions",
                    ["function"],
                    "No Lambda functions were found."
                ),
                tab(
                    "versions",
                    "Versions",
                    ["version"],
                    "No Lambda versions are loaded."
                ),
                tab(
                    "aliases",
                    "Aliases",
                    ["alias"],
                    "No Lambda aliases are loaded."
                ),
                tab(
                    "event-sources",
                    "Event Sources",
                    ["event-source-mapping"],
                    "No Lambda event source mappings are loaded."
                ),
                tab(
                    "environment",
                    "Environment",
                    ["configuration"],
                    "Configuration summaries exclude environment variable values."
                )
            ]),
            json!([
                resource(
                    "function/process-order",
                    "process-order",
                    "function",
                    "active",
                    json!({"runtime": "provided.al2023", "handler": "bootstrap", "memory_size": "128"})
                ),
                resource(
                    "version/process-order/1",
                    "1",
                    "version",
                    "active",
                    json!({"function_name": "process-order"})
                ),
                resource(
                    "alias/process-order/live",
                    "live",
                    "alias",
                    "available",
                    json!({"function_name": "process-order", "function_version": "1"})
                ),
                resource(
                    "event-source-mapping/local-orders",
                    "local-orders",
                    "event-source-mapping",
                    "enabled",
                    json!({"function_arn": "arn:aws:lambda:us-east-1:000000000000:function:process-order"})
                ),
                resource(
                    "configuration/process-order",
                    "process-order configuration",
                    "configuration",
                    "loaded",
                    json!({"environment": "hidden", "timeout": "30"})
                )
            ]),
            json!([
                "update_function_code",
                "publish_version",
                "create_alias",
                "delete_alias"
            ]),
        ),
        "ec2" => (
            "EC2",
            "managed",
            json!([
                tab(
                    "instances",
                    "Instances",
                    ["instance"],
                    "No EC2 instances were found."
                ),
                tab("vpcs", "VPCs", ["vpc"], "No EC2 VPCs are loaded."),
                tab(
                    "subnets",
                    "Subnets",
                    ["subnet"],
                    "No EC2 subnets are loaded."
                ),
                tab(
                    "security-groups",
                    "Security Groups",
                    ["security-group"],
                    "No EC2 security groups are loaded."
                ),
                tab(
                    "key-pairs",
                    "Key Pairs",
                    ["key-pair"],
                    "No EC2 key pairs are loaded."
                ),
                tab(
                    "images",
                    "Images",
                    ["image"],
                    "No local EC2 images are loaded."
                ),
                tab(
                    "volumes",
                    "Volumes",
                    ["volume"],
                    "No EC2 volumes are loaded."
                )
            ]),
            json!([
                resource(
                    "instance/i-local001",
                    "dev-runner",
                    "instance",
                    "running",
                    json!({"instance_type": "t3.micro", "vpc_id": "vpc-local"})
                ),
                resource(
                    "vpc/vpc-local",
                    "vpc-local",
                    "vpc",
                    "available",
                    json!({"cidr_block": "10.0.0.0/16"})
                ),
                resource(
                    "subnet/subnet-local",
                    "subnet-local",
                    "subnet",
                    "available",
                    json!({"availability_zone": "us-east-1a"})
                ),
                resource(
                    "security-group/sg-local",
                    "local-access",
                    "security-group",
                    "available",
                    json!({"inbound_rules": "2"})
                ),
                resource(
                    "key-pair/local-key",
                    "local-key",
                    "key-pair",
                    "available",
                    json!({"fingerprint": "local"})
                ),
                resource(
                    "image/ami-local",
                    "floci-local",
                    "image",
                    "available",
                    json!({"architecture": "x86_64"})
                ),
                resource(
                    "volume/vol-local",
                    "vol-local",
                    "volume",
                    "available",
                    json!({"size_gib": "8"})
                )
            ]),
            json!([
                "create_instance",
                "modify_instance",
                "delete_security_group"
            ]),
        ),
        "ecs" => (
            "ECS",
            "managed",
            json!([
                tab(
                    "clusters",
                    "Clusters",
                    ["cluster"],
                    "No ECS clusters were found."
                ),
                tab(
                    "services",
                    "Services",
                    ["service"],
                    "No ECS services are loaded."
                ),
                tab("tasks", "Tasks", ["task"], "No ECS tasks are loaded."),
                tab(
                    "task-definitions",
                    "Task Definitions",
                    ["task-definition"],
                    "No ECS task definitions are loaded."
                ),
                tab(
                    "container-instances",
                    "Container Instances",
                    ["container-instance"],
                    "No ECS container instances are loaded."
                )
            ]),
            json!([
                resource(
                    "cluster/arn:aws:ecs:us-east-1:000000000000:cluster/default",
                    "default",
                    "cluster",
                    "active",
                    json!({"running_tasks": "1", "active_services": "1"})
                ),
                resource(
                    "service/arn:aws:ecs:us-east-1:000000000000:service/default/orders-api",
                    "orders-api",
                    "service",
                    "active",
                    json!({"desired_count": "1", "running_count": "1"})
                ),
                resource(
                    "task/arn:aws:ecs:us-east-1:000000000000:task/default/manual",
                    "manual",
                    "task",
                    "running",
                    json!({"task_definition": "orders-worker:1"})
                ),
                resource(
                    "task-definition/orders-worker:1",
                    "orders-worker:1",
                    "task-definition",
                    "registered",
                    json!({"task_definition_arn": "orders-worker:1"})
                ),
                resource(
                    "container-instance/local",
                    "local",
                    "container-instance",
                    "active",
                    json!({"running_tasks": "1"})
                )
            ]),
            json!(["create_service", "delete_cluster"]),
        ),
        "eks" => read_only_preview(
            "EKS",
            json!([
                tab(
                    "clusters",
                    "Clusters",
                    ["cluster"],
                    "No EKS clusters were found."
                ),
                tab(
                    "node-groups",
                    "Node Groups",
                    ["node-group"],
                    "No EKS node groups are loaded."
                ),
                tab("addons", "Add-ons", ["addon"], "No EKS add-ons are loaded.")
            ]),
            json!([
                resource(
                    "cluster/local-eks",
                    "local-eks",
                    "cluster",
                    "active",
                    json!({"endpoint": "https://local-eks.test", "version": "1.30"})
                ),
                resource(
                    "node-group/local-eks/default",
                    "default",
                    "node-group",
                    "available",
                    json!({"cluster_name": "local-eks"})
                ),
                resource(
                    "addon/local-eks/coredns",
                    "coredns",
                    "addon",
                    "available",
                    json!({"cluster_name": "local-eks"})
                )
            ]),
        ),
        "ecr" => (
            "ECR",
            "managed",
            json!([
                tab(
                    "repositories",
                    "Repositories",
                    ["repository"],
                    "No ECR repositories were found."
                ),
                tab("images", "Images", ["image"], "No ECR images are loaded."),
                tab("tags", "Tags", ["tag"], "No ECR image tags are loaded.")
            ]),
            json!([
                resource(
                    "repository/orders-api",
                    "orders-api",
                    "repository",
                    "available",
                    json!({"repository_uri": "000000000000.dkr.ecr.us-east-1.local/orders-api"})
                ),
                resource(
                    "image/orders-api/sha256:local",
                    "latest",
                    "image",
                    "available",
                    json!({"repository_name": "orders-api", "image_digest": "sha256:local"})
                ),
                resource(
                    "tag/orders-api/latest",
                    "latest",
                    "tag",
                    "available",
                    json!({"repository_name": "orders-api"})
                )
            ]),
            json!(["put_lifecycle_policy", "start_image_scan"]),
        ),
        "msk" => read_only_preview(
            "MSK",
            json!([
                tab(
                    "clusters",
                    "Clusters",
                    ["cluster"],
                    "No MSK clusters were found."
                ),
                tab(
                    "brokers",
                    "Brokers",
                    ["broker"],
                    "No MSK brokers are loaded."
                ),
                tab(
                    "configurations",
                    "Configurations",
                    ["configuration"],
                    "No MSK configurations are loaded."
                )
            ]),
            json!([
                resource(
                    "cluster/arn:aws:kafka:us-east-1:000000000000:cluster/local",
                    "local",
                    "cluster",
                    "active",
                    json!({"cluster_type": "provisioned"})
                ),
                resource(
                    "broker/local/broker-1",
                    "broker-1",
                    "broker",
                    "available",
                    json!({"client_vpc_ip": "10.0.0.10"})
                ),
                resource(
                    "configuration/local",
                    "local",
                    "configuration",
                    "available",
                    json!({"arn": "configuration/local"})
                )
            ]),
        ),
        "codebuild" => (
            "CodeBuild",
            "managed",
            json!([
                tab(
                    "projects",
                    "Projects",
                    ["project"],
                    "No CodeBuild projects were found."
                ),
                tab(
                    "builds",
                    "Builds",
                    ["build"],
                    "No CodeBuild builds are loaded."
                ),
                tab(
                    "reports",
                    "Reports",
                    ["report"],
                    "No CodeBuild reports are loaded."
                )
            ]),
            json!([
                resource(
                    "project/floci-ui",
                    "floci-ui",
                    "project",
                    "available",
                    json!({"source_type": "NO_SOURCE", "environment_type": "LINUX_CONTAINER"})
                ),
                resource(
                    "build/floci-ui:1",
                    "#1",
                    "build",
                    "succeeded",
                    json!({"project_name": "floci-ui"})
                ),
                resource(
                    "report/floci-ui-tests",
                    "floci-ui-tests",
                    "report",
                    "available",
                    json!({"report_group_arn": "floci-ui-tests"})
                )
            ]),
            json!(["delete_project", "retry_build"]),
        ),
        "codedeploy" => (
            "CodeDeploy",
            "managed",
            json!([
                tab(
                    "applications",
                    "Applications",
                    ["application"],
                    "No CodeDeploy applications were found."
                ),
                tab(
                    "deployment-groups",
                    "Deployment Groups",
                    ["deployment-group"],
                    "No CodeDeploy deployment groups are loaded."
                ),
                tab(
                    "deployments",
                    "Deployments",
                    ["deployment"],
                    "No CodeDeploy deployments are loaded."
                )
            ]),
            json!([
                resource(
                    "application/orders-api",
                    "orders-api",
                    "application",
                    "available",
                    json!({})
                ),
                resource(
                    "deployment-group/orders-api/local",
                    "local",
                    "deployment-group",
                    "available",
                    json!({"application_name": "orders-api"})
                ),
                resource(
                    "deployment/d-local",
                    "d-local",
                    "deployment",
                    "succeeded",
                    json!({"application_name": "orders-api", "deployment_group_name": "local"})
                )
            ]),
            json!(["delete_application", "delete_deployment_group"]),
        ),
        "autoscaling" => (
            "Auto Scaling",
            "managed",
            json!([
                tab(
                    "groups",
                    "Groups",
                    ["auto-scaling-group"],
                    "No Auto Scaling groups were found."
                ),
                tab(
                    "instances",
                    "Instances",
                    ["scaling-instance"],
                    "No Auto Scaling instances are loaded."
                ),
                tab(
                    "policies",
                    "Policies",
                    ["scaling-policy"],
                    "No Auto Scaling policies are loaded."
                ),
                tab(
                    "lifecycle-hooks",
                    "Lifecycle Hooks",
                    ["lifecycle-hook"],
                    "No Auto Scaling lifecycle hooks are loaded."
                ),
                tab(
                    "launch-configurations",
                    "Launch Configurations",
                    ["launch-configuration"],
                    "No launch configurations are loaded."
                )
            ]),
            json!([
                resource(
                    "auto-scaling-group/orders-workers",
                    "orders-workers",
                    "auto-scaling-group",
                    "available",
                    json!({"min_size": "1", "max_size": "4", "desired_capacity": "2"})
                ),
                resource(
                    "scaling-instance/orders-workers/i-local001",
                    "i-local001",
                    "scaling-instance",
                    "healthy",
                    json!({"auto_scaling_group": "orders-workers"})
                ),
                resource(
                    "scaling-policy/scale-out",
                    "scale-out",
                    "scaling-policy",
                    "available",
                    json!({"auto_scaling_group": "orders-workers"})
                ),
                resource(
                    "lifecycle-hook/orders-workers/drain",
                    "drain",
                    "lifecycle-hook",
                    "available",
                    json!({"auto_scaling_group": "orders-workers"})
                ),
                resource(
                    "launch-configuration/orders-workers",
                    "orders-workers",
                    "launch-configuration",
                    "available",
                    json!({"instance_type": "t3.micro"})
                )
            ]),
            json!([
                "create_auto_scaling_group",
                "delete_policy",
                "delete_lifecycle_hook"
            ]),
        ),
        "bedrockruntime" => (
            "Bedrock Runtime",
            "managed",
            json!([
                tab(
                    "models",
                    "Models",
                    ["model"],
                    "No local Bedrock Runtime metadata is loaded."
                ),
                tab(
                    "invocations",
                    "Invocations",
                    ["invocation"],
                    "No invocation summaries are loaded."
                ),
                tab(
                    "request-template",
                    "Request Template",
                    ["request-template"],
                    "No request templates are loaded."
                ),
                tab(
                    "response-preview",
                    "Response Preview",
                    ["response-preview"],
                    "No response preview is available."
                )
            ]),
            json!([
                resource(
                    "model/local-bedrock-runtime",
                    "local-bedrock-runtime",
                    "model",
                    "configured",
                    json!({"execution_context": "Local Floci endpoint only"})
                ),
                resource(
                    "request-template/json-invoke",
                    "JSON invoke template",
                    "request-template",
                    "available",
                    json!({"template": "{\"prompt\":\"hello from floci-ui\"}"})
                ),
                resource(
                    "response-preview/latest",
                    "Latest response preview",
                    "response-preview",
                    "empty",
                    json!({"large_outputs": "summarized"})
                )
            ]),
            json!(["stream_invoke_model"]),
        ),
        "iam" => (
            "IAM",
            "managed",
            json!([
                tab("users", "Users", ["user"], "No IAM users were found."),
                tab("roles", "Roles", ["role"], "No IAM roles were found."),
                tab("groups", "Groups", ["group"], "No IAM groups were found."),
                tab(
                    "policies",
                    "Policies",
                    ["policy"],
                    "No IAM policies were found."
                ),
                tab(
                    "access-keys",
                    "Access Keys",
                    ["access-key"],
                    "No IAM access keys were found."
                ),
                tab(
                    "instance-profiles",
                    "Instance Profiles",
                    ["instance-profile"],
                    "No IAM instance profiles were found."
                )
            ]),
            json!([
                resource(
                    "user/floci-admin",
                    "floci-admin",
                    "user",
                    "active",
                    json!({"arn": "arn:aws:iam::000000000000:user/floci-admin"})
                ),
                resource(
                    "role/floci-lambda-local",
                    "floci-lambda-local",
                    "role",
                    "available",
                    json!({"arn": "arn:aws:iam::000000000000:role/floci-lambda-local"})
                ),
                resource(
                    "group/local-operators",
                    "local-operators",
                    "group",
                    "available",
                    json!({"path": "/"})
                ),
                resource(
                    "policy/arn:aws:iam::000000000000:policy/floci-local",
                    "floci-local",
                    "policy",
                    "available",
                    json!({"default_version_id": "v1", "attachment_count": "1"})
                ),
                resource(
                    "access-key/floci-admin/AKIALOCAL",
                    "AKIALOCAL",
                    "access-key",
                    "Active",
                    json!({"user_name": "floci-admin", "secret_access_key": "redacted"})
                ),
                resource(
                    "instance-profile/floci-local-profile",
                    "floci-local-profile",
                    "instance-profile",
                    "available",
                    json!({"roles": "floci-lambda-local"})
                )
            ]),
            json!(["attach_policy", "detach_policy", "delete_group"]),
        ),
        "sts" => read_only_preview(
            "STS",
            json!([
                tab(
                    "caller-identity",
                    "Caller Identity",
                    ["caller-identity"],
                    "No STS caller identity is available."
                ),
                tab(
                    "sessions",
                    "Session Context",
                    ["session"],
                    "No local session context is available."
                )
            ]),
            json!([
                resource(
                    "caller-identity/current",
                    "000000000000",
                    "caller-identity",
                    "resolved",
                    json!({"account": "000000000000", "arn": "arn:aws:sts::000000000000:assumed-role/local/floci", "credentials": "redacted"})
                ),
                resource(
                    "session/local",
                    "Local session context",
                    "session",
                    "local",
                    json!({"region": "us-east-1", "access_key_id": "redacted", "secret_access_key": "redacted"})
                )
            ]),
        ),
        "cognito" => (
            "Cognito",
            "managed",
            json!([
                tab(
                    "user-pools",
                    "User Pools",
                    ["user-pool"],
                    "No Cognito user pools were found."
                ),
                tab(
                    "app-clients",
                    "App Clients",
                    ["app-client"],
                    "No Cognito app clients were found."
                ),
                tab("users", "Users", ["user"], "No Cognito users were found."),
                tab(
                    "groups",
                    "Groups",
                    ["group"],
                    "No Cognito groups were found."
                ),
                tab(
                    "providers",
                    "Providers",
                    ["identity-provider"],
                    "No Cognito identity providers were found."
                )
            ]),
            json!([
                resource(
                    "user-pool/us-east-1_local",
                    "floci-local-users",
                    "user-pool",
                    "Enabled",
                    json!({"user_pool_id": "us-east-1_local"})
                ),
                resource(
                    "app-client/us-east-1_local/local-client",
                    "local-client",
                    "app-client",
                    "available",
                    json!({"user_pool_id": "us-east-1_local", "client_secret": "redacted"})
                ),
                resource(
                    "user/us-east-1_local/admin",
                    "admin",
                    "user",
                    "CONFIRMED",
                    json!({"user_pool_id": "us-east-1_local"})
                ),
                resource(
                    "group/us-east-1_local/operators",
                    "operators",
                    "group",
                    "available",
                    json!({"user_pool_id": "us-east-1_local"})
                ),
                resource(
                    "identity-provider/us-east-1_local/Google",
                    "Google",
                    "identity-provider",
                    "available",
                    json!({"provider_type": "Google"})
                )
            ]),
            json!(["create_group", "delete_group"]),
        ),
        "kms" => (
            "KMS",
            "managed",
            json!([
                tab("keys", "Keys", ["key"], "No KMS keys were found."),
                tab(
                    "aliases",
                    "Aliases",
                    ["alias"],
                    "No KMS aliases were found."
                ),
                tab("grants", "Grants", ["grant"], "No KMS grants were found."),
                tab(
                    "policies",
                    "Policies",
                    ["key-policy"],
                    "KMS key policies are summarized through selected key metadata."
                )
            ]),
            json!([
                resource(
                    "key/11111111-1111-1111-1111-111111111111",
                    "11111111-1111-1111-1111-111111111111",
                    "key",
                    "Enabled",
                    json!({"key_usage": "ENCRYPT_DECRYPT", "key_material": "not-exportable"})
                ),
                resource(
                    "alias/alias/floci-local",
                    "alias/floci-local",
                    "alias",
                    "active",
                    json!({"target_key_id": "11111111-1111-1111-1111-111111111111"})
                ),
                resource(
                    "grant/11111111-1111-1111-1111-111111111111/local-grant",
                    "local-grant",
                    "grant",
                    "active",
                    json!({"operations": "Encrypt, Decrypt"})
                )
            ]),
            json!(["create_grant", "delete_alias", "put_key_policy"]),
        ),
        "secretsmanager" => (
            "Secrets Manager",
            "managed",
            json!([
                tab("secrets", "Secrets", ["secret"], "No secrets were found."),
                tab(
                    "versions",
                    "Versions",
                    ["secret-version"],
                    "No secret versions were found."
                ),
                tab(
                    "rotation",
                    "Rotation",
                    ["rotation"],
                    "No secret rotation metadata is available."
                ),
                tab(
                    "tags",
                    "Tags",
                    ["tag-set"],
                    "Secret tags are folded into resource metadata."
                )
            ]),
            json!([
                resource(
                    "secret/arn:aws:secretsmanager:us-east-1:000000000000:secret:orders/api",
                    "orders/api",
                    "secret",
                    "active",
                    json!({"secret_value": "Hidden until explicit reveal; value is never included in inventory.", "rotation_enabled": "false"})
                ),
                resource(
                    "secret-version/arn:aws:secretsmanager:us-east-1:000000000000:secret:orders/api/version-1",
                    "version-1",
                    "secret-version",
                    "current",
                    json!({"version_stages": "AWSCURRENT", "secret_value": "Hidden until explicit reveal; value is never included in inventory."})
                ),
                resource(
                    "rotation/arn:aws:secretsmanager:us-east-1:000000000000:secret:orders/api",
                    "orders/api rotation",
                    "rotation",
                    "disabled",
                    json!({"rotation_lambda_arn": "not configured"})
                )
            ]),
            json!(["create_secret", "put_secret_value", "delete_secret"]),
        ),
        "ssm" => (
            "SSM",
            "managed",
            json!([
                tab(
                    "parameters",
                    "Parameters",
                    ["parameter"],
                    "No SSM parameters were found."
                ),
                tab(
                    "documents",
                    "Documents",
                    ["document"],
                    "No SSM documents were found."
                ),
                tab(
                    "commands",
                    "Command Invocations",
                    ["command"],
                    "No SSM command invocations were found."
                ),
                tab(
                    "managed-instances",
                    "Managed Instances",
                    ["managed-instance"],
                    "No SSM managed instances were found."
                )
            ]),
            json!([
                resource(
                    "parameter//floci/db/password",
                    "/floci/db/password",
                    "parameter",
                    "SecureString",
                    json!({"value": "Hidden until explicit reveal; value is never included in inventory.", "type": "SecureString"})
                ),
                resource(
                    "document/AWS-RunShellScript",
                    "AWS-RunShellScript",
                    "document",
                    "available",
                    json!({"document_type": "Command"})
                ),
                resource(
                    "command/cmd-local/i-local001",
                    "cmd-local",
                    "command",
                    "Success",
                    json!({"instance_id": "i-local001"})
                ),
                resource(
                    "managed-instance/i-local001",
                    "i-local001",
                    "managed-instance",
                    "Online",
                    json!({"agent_version": "local"})
                )
            ]),
            json!(["put_parameter", "delete_parameter", "send_command"]),
        ),
        "appconfig" => (
            "AppConfig",
            "managed",
            json!([
                tab(
                    "applications",
                    "Applications",
                    ["application"],
                    "No AppConfig applications were found."
                ),
                tab(
                    "environments",
                    "Environments",
                    ["environment"],
                    "No AppConfig environments were found."
                ),
                tab(
                    "profiles",
                    "Profiles",
                    ["configuration-profile"],
                    "No AppConfig configuration profiles were found."
                ),
                tab(
                    "versions",
                    "Versions",
                    ["hosted-version"],
                    "No hosted configuration versions were found."
                ),
                tab(
                    "deployments",
                    "Deployments",
                    ["deployment"],
                    "No AppConfig deployments were found."
                )
            ]),
            json!([
                resource(
                    "application/app-local",
                    "floci-local-config",
                    "application",
                    "available",
                    json!({})
                ),
                resource(
                    "environment/app-local/env-local",
                    "local",
                    "environment",
                    "ReadyForDeployment",
                    json!({"application_id": "app-local"})
                ),
                resource(
                    "configuration-profile/app-local/profile-local",
                    "runtime-config",
                    "configuration-profile",
                    "available",
                    json!({"location_uri": "hosted"})
                ),
                resource(
                    "hosted-version/app-local/profile-local/1",
                    "v1",
                    "hosted-version",
                    "available",
                    json!({"content": "metadata only", "content_type": "application/json"})
                ),
                resource(
                    "deployment/app-local/env-local/1",
                    "#1",
                    "deployment",
                    "Complete",
                    json!({"configuration_version": "1"})
                )
            ]),
            json!(["delete_application", "delete_environment"]),
        ),
        "appconfigdata" => read_only_preview(
            "AppConfig Data",
            json!([
                tab(
                    "session-preview",
                    "Session Preview",
                    ["session-preview"],
                    "No AppConfigData session preview is loaded."
                ),
                tab(
                    "configuration-preview",
                    "Configuration Preview",
                    ["configuration-preview"],
                    "No AppConfigData configuration preview is loaded."
                )
            ]),
            json!([
                resource(
                    "session-preview/local",
                    "Local session preview",
                    "session-preview",
                    "metadata-only",
                    json!({"token_storage": "configuration tokens are not persisted in the UI"})
                ),
                resource(
                    "configuration-preview/local",
                    "Configuration payload preview",
                    "configuration-preview",
                    "redacted",
                    json!({"payload": "not fetched until an explicit AppConfigData session is supported"})
                )
            ]),
        ),
        "acm" => (
            "ACM",
            "managed",
            json!([
                tab(
                    "certificates",
                    "Certificates",
                    ["certificate"],
                    "No ACM certificates were found."
                ),
                tab(
                    "domains",
                    "Domains",
                    ["domain"],
                    "No ACM certificate domains were found."
                ),
                tab(
                    "validation",
                    "Validation",
                    ["validation"],
                    "No ACM validation metadata was found."
                ),
                tab(
                    "tags",
                    "Tags",
                    ["tag-set"],
                    "ACM tags are folded into resource metadata."
                )
            ]),
            json!([
                resource(
                    "certificate/arn:aws:acm:us-east-1:000000000000:certificate/local",
                    "api.local.floci",
                    "certificate",
                    "ISSUED",
                    json!({"certificate_arn": "arn:aws:acm:us-east-1:000000000000:certificate/local", "private_key_material": "not loaded"})
                ),
                resource(
                    "domain/arn:aws:acm:us-east-1:000000000000:certificate/local/api.local.floci",
                    "api.local.floci",
                    "domain",
                    "covered",
                    json!({"certificate_arn": "arn:aws:acm:us-east-1:000000000000:certificate/local"})
                ),
                resource(
                    "validation/arn:aws:acm:us-east-1:000000000000:certificate/local/api.local.floci",
                    "api.local.floci",
                    "validation",
                    "SUCCESS",
                    json!({"validation_method": "DNS", "record_value": "local-validation-token"})
                )
            ]),
            json!([
                "import_certificate",
                "delete_certificate",
                "export_certificate"
            ]),
        ),
        "apigateway" => (
            "API Gateway",
            "managed",
            json!([
                tab(
                    "rest-apis",
                    "REST APIs",
                    ["rest-api"],
                    "No API Gateway REST APIs were found."
                ),
                tab(
                    "resources",
                    "Resources",
                    ["resource"],
                    "No REST API resources are loaded."
                ),
                tab(
                    "methods",
                    "Methods",
                    ["method"],
                    "No REST API methods are loaded."
                ),
                tab(
                    "integrations",
                    "Integrations",
                    ["integration"],
                    "No REST API integrations are loaded."
                ),
                tab(
                    "deployments",
                    "Deployments",
                    ["deployment"],
                    "No REST API deployments are loaded."
                ),
                tab(
                    "stages",
                    "Stages",
                    ["stage"],
                    "No REST API stages are loaded."
                )
            ]),
            json!([
                resource(
                    "rest-api/api-local",
                    "orders-rest-api",
                    "rest-api",
                    "available",
                    json!({"endpoint": "http://localhost:4566/restapis/api-local/local/_user_request_", "resources": "2"})
                ),
                resource(
                    "resource/api-local/root",
                    "/",
                    "resource",
                    "available",
                    json!({"api_id": "api-local", "path": "/"})
                ),
                resource(
                    "resource/api-local/orders",
                    "/orders",
                    "resource",
                    "available",
                    json!({"api_id": "api-local", "path": "/orders"})
                ),
                resource(
                    "method/api-local/orders/GET",
                    "GET /orders",
                    "method",
                    "configured",
                    json!({"authorization": "NONE", "api_id": "api-local"})
                ),
                resource(
                    "integration/api-local/orders/GET",
                    "GET /orders integration",
                    "integration",
                    "configured",
                    json!({"type": "HTTP_PROXY", "uri": "http://localhost:3000/orders", "credentials": "redacted"})
                ),
                resource(
                    "deployment/api-local/local",
                    "local",
                    "deployment",
                    "deployed",
                    json!({"stage": "local"})
                ),
                resource(
                    "stage/api-local/local",
                    "local",
                    "stage",
                    "available",
                    json!({"deployment_id": "preview"})
                )
            ]),
            json!([]),
        ),
        "apigatewayv2" => (
            "API Gateway v2",
            "managed",
            json!([
                tab(
                    "apis",
                    "APIs",
                    ["api"],
                    "No API Gateway v2 APIs were found."
                ),
                tab(
                    "routes",
                    "Routes",
                    ["route"],
                    "No API Gateway v2 routes are loaded."
                ),
                tab(
                    "integrations",
                    "Integrations",
                    ["integration"],
                    "No API Gateway v2 integrations are loaded."
                ),
                tab(
                    "deployments",
                    "Deployments",
                    ["deployment"],
                    "No API Gateway v2 deployments are loaded."
                ),
                tab(
                    "stages",
                    "Stages",
                    ["stage"],
                    "No API Gateway v2 stages are loaded."
                ),
                tab(
                    "authorizers",
                    "Authorizers",
                    ["authorizer"],
                    "No API Gateway v2 authorizers are loaded."
                )
            ]),
            json!([
                resource(
                    "api/http-api-local",
                    "orders-http-api",
                    "api",
                    "available",
                    json!({"protocol": "HTTP", "endpoint": "http://localhost:4566/_localstack/apigateway/http-api-local"})
                ),
                resource(
                    "route/http-api-local/GET-orders",
                    "GET /orders",
                    "route",
                    "configured",
                    json!({"route_key": "GET /orders", "target": "integrations/int-local"})
                ),
                resource(
                    "integration/http-api-local/int-local",
                    "orders-service",
                    "integration",
                    "configured",
                    json!({"method": "POST", "uri": "http://localhost:3000/orders"})
                ),
                resource(
                    "deployment/http-api-local/dep-local",
                    "dep-local",
                    "deployment",
                    "deployed",
                    json!({"auto_deployed": "false"})
                ),
                resource(
                    "stage/http-api-local/local",
                    "local",
                    "stage",
                    "available",
                    json!({"auto_deploy": "true"})
                ),
                resource(
                    "authorizer/http-api-local/auth-local",
                    "local-jwt",
                    "authorizer",
                    "configured",
                    json!({"identity_sources": "redacted"})
                )
            ]),
            json!([]),
        ),
        "elbv2" => (
            "Elastic Load Balancing v2",
            "managed",
            json!([
                tab(
                    "load-balancers",
                    "Load Balancers",
                    ["load-balancer"],
                    "No load balancers were found."
                ),
                tab(
                    "listeners",
                    "Listeners",
                    ["listener"],
                    "No listeners are loaded."
                ),
                tab(
                    "rules",
                    "Rules",
                    ["listener-rule"],
                    "No listener rules are loaded."
                ),
                tab(
                    "target-groups",
                    "Target Groups",
                    ["target-group"],
                    "No target groups were found."
                ),
                tab(
                    "targets",
                    "Targets",
                    ["target"],
                    "No target health rows are loaded."
                )
            ]),
            json!([
                resource(
                    "load-balancer/app/orders-local/50dc6c495c0c9188",
                    "orders-local",
                    "load-balancer",
                    "active",
                    json!({"scheme": "internal", "dns_name": "orders-local.localhost.localstack.cloud"})
                ),
                resource(
                    "listener/app/orders-local/80",
                    "HTTP :80",
                    "listener",
                    "active",
                    json!({"protocol": "HTTP", "port": "80"})
                ),
                resource(
                    "listener-rule/app/orders-local/80/default",
                    "default",
                    "listener-rule",
                    "active",
                    json!({"priority": "default"})
                ),
                resource(
                    "target-group/orders-targets",
                    "orders-targets",
                    "target-group",
                    "available",
                    json!({"protocol": "HTTP", "port": "8080", "targets": "1"})
                ),
                resource(
                    "target/orders-targets/i-00000000000000000",
                    "i-00000000000000000",
                    "target",
                    "healthy",
                    json!({"port": "8080", "target_group": "orders-targets"})
                )
            ]),
            json!([]),
        ),
        "route53" => (
            "Route 53",
            "managed",
            json!([
                tab(
                    "hosted-zones",
                    "Hosted Zones",
                    ["hosted-zone"],
                    "No hosted zones were found."
                ),
                tab(
                    "records",
                    "Records",
                    ["record-set"],
                    "No record sets are loaded."
                ),
                tab(
                    "health-checks",
                    "Health Checks",
                    ["health-check"],
                    "No health checks are loaded."
                ),
                tab(
                    "changes",
                    "Changes",
                    ["change-batch"],
                    "No change batches are loaded."
                )
            ]),
            json!([
                resource(
                    "hosted-zone/ZLOCALFLOCI",
                    "local.floci.test.",
                    "hosted-zone",
                    "available",
                    json!({"records": "3", "private": "false"})
                ),
                resource(
                    "record-set/ZLOCALFLOCI/api.local.floci.test./A",
                    "api.local.floci.test.",
                    "record-set",
                    "INSYNC",
                    json!({"type": "A", "ttl": "300", "value": "127.0.0.1"})
                ),
                resource(
                    "health-check/local-api",
                    "local-api",
                    "health-check",
                    "healthy",
                    json!({"type": "HTTP", "target": "api.local.floci.test."})
                ),
                resource(
                    "change-batch/ZLOCALFLOCI/preview",
                    "preview change",
                    "change-batch",
                    "INSYNC",
                    json!({"submitted_by": "browser preview"})
                )
            ]),
            json!([]),
        ),
        "transfer" => (
            "Transfer Family",
            "managed",
            json!([
                tab(
                    "servers",
                    "Servers",
                    ["server"],
                    "No Transfer servers were found."
                ),
                tab("users", "Users", ["user"], "No Transfer users are loaded."),
                tab(
                    "workflows",
                    "Workflows",
                    ["workflow"],
                    "No workflows are loaded."
                ),
                tab(
                    "host-keys",
                    "Host Keys",
                    ["host-key"],
                    "No host keys are loaded."
                )
            ]),
            json!([
                resource(
                    "server/s-1234567890abcdef0",
                    "s-1234567890abcdef0",
                    "server",
                    "ONLINE",
                    json!({"protocols": "SFTP", "endpoint_type": "PUBLIC"})
                ),
                resource(
                    "user/s-1234567890abcdef0/deploy",
                    "deploy",
                    "user",
                    "available",
                    json!({"server_id": "s-1234567890abcdef0", "role": "redacted", "home_directory": "redacted"})
                ),
                resource(
                    "workflow/w-1234567890abcdef0",
                    "post-upload-audit",
                    "workflow",
                    "available",
                    json!({"steps": "1"})
                ),
                resource(
                    "host-key/s-1234567890abcdef0/key-local",
                    "key-local",
                    "host-key",
                    "active",
                    json!({"fingerprint": "redacted"})
                )
            ]),
            json!([]),
        ),
        "cloudwatchlogs" => (
            "CloudWatch Logs",
            "managed",
            json!([
                tab(
                    "log-groups",
                    "Log Groups",
                    ["log-group"],
                    "No log groups were found."
                ),
                tab(
                    "streams",
                    "Streams",
                    ["log-stream"],
                    "No log streams are loaded."
                ),
                tab(
                    "events",
                    "Events",
                    ["log-event"],
                    "Run Tail events on a log group to fetch recent events."
                ),
                tab(
                    "filters",
                    "Filters",
                    ["metric-filter"],
                    "No metric filters are loaded."
                ),
                tab(
                    "subscriptions",
                    "Subscriptions",
                    ["subscription-filter"],
                    "No subscription filters are loaded."
                )
            ]),
            json!([
                resource(
                    "log-group//aws/lambda/orders-worker",
                    "/aws/lambda/orders-worker",
                    "log-group",
                    "active",
                    json!({"retention_in_days": "7", "stored_bytes": "4096"})
                ),
                resource(
                    "log-stream//aws/lambda/orders-worker/2026/05/16/[$LATEST]preview",
                    "2026/05/16/[$LATEST]preview",
                    "log-stream",
                    "active",
                    json!({"log_group": "/aws/lambda/orders-worker", "events": "3"})
                ),
                resource(
                    "log-event//aws/lambda/orders-worker/preview/1",
                    "redacted event preview",
                    "log-event",
                    "available",
                    json!({"message": "redacted", "timestamp": "preview"})
                ),
                resource(
                    "metric-filter//aws/lambda/orders-worker/errors",
                    "errors",
                    "metric-filter",
                    "active",
                    json!({"pattern": "ERROR"})
                ),
                resource(
                    "subscription-filter//aws/lambda/orders-worker/audit",
                    "audit",
                    "subscription-filter",
                    "active",
                    json!({"destination": "arn:aws:lambda:us-east-1:000000000000:function:audit"})
                )
            ]),
            json!([]),
        ),
        "cloudwatch" => (
            "CloudWatch",
            "managed",
            json!([
                tab(
                    "metrics",
                    "Metrics",
                    ["metric"],
                    "No CloudWatch metrics were found."
                ),
                tab("alarms", "Alarms", ["alarm"], "No alarms are loaded."),
                tab(
                    "dashboards",
                    "Dashboards",
                    ["dashboard"],
                    "No dashboards are loaded."
                ),
                tab(
                    "query-results",
                    "Query Results",
                    ["query-result"],
                    "Run Query metric on a metric to fetch datapoints."
                )
            ]),
            json!([
                resource(
                    "metric/AWS/ApiGateway/Count",
                    "AWS/ApiGateway Count",
                    "metric",
                    "available",
                    json!({"namespace": "AWS/ApiGateway", "dimensions": "ApiName=orders-http-api"})
                ),
                resource(
                    "alarm/high-latency",
                    "high-latency",
                    "alarm",
                    "OK",
                    json!({"metric": "Latency", "threshold": "1"})
                ),
                resource(
                    "dashboard/local-network",
                    "local-network",
                    "dashboard",
                    "available",
                    json!({"widgets": "2"})
                ),
                resource(
                    "query-result/AWS/ApiGateway/Count/latest",
                    "latest Count datapoints",
                    "query-result",
                    "available",
                    json!({"period": "60", "datapoints": "5"})
                )
            ]),
            json!([]),
        ),
        _ => return browser_preview_unsupported_inventory(service_key),
    };

    Some(json!({
        "service_key": service_key,
        "service_label": label,
        "support_level": support_level,
        "refreshed_at": browser_preview_timestamp(),
        "tabs": tabs,
        "resources": resources,
        "unsupported_operations": unsupported_operations
    }))
}

#[cfg(all(target_arch = "wasm32", debug_assertions))]
fn browser_preview_unsupported_inventory(service_key: &str) -> Option<serde_json::Value> {
    let service = preview_service_descriptor(service_key)?;
    let tabs = service
        .primary_resource_kinds
        .iter()
        .map(|kind| {
            json!({
                "key": kind,
                "label": title_case_kind(kind),
                "kinds": [kind],
                "empty_message": format!(
                    "{} `{kind}` inventory is not available until an adapter is implemented.",
                    service.label
                )
            })
        })
        .collect::<Vec<_>>();

    Some(json!({
        "service_key": service.key,
        "service_label": service.label,
        "support_level": service.support_level,
        "refreshed_at": browser_preview_timestamp(),
        "tabs": tabs,
        "resources": [],
        "unsupported_operations": ["list", "inspect", "create", "update", "delete"]
    }))
}

#[cfg(all(target_arch = "wasm32", debug_assertions))]
fn browser_preview_resource_detail(args: Option<&serde_json::Value>) -> Option<serde_json::Value> {
    let service_key = preview_request_value(args, "service_key")?;
    let resource_id = preview_request_value(args, "resource_id")?;
    let inventory = browser_preview_inventory(Some(&json!({
        "request": {
            "service_key": service_key
        }
    })))?;
    let summary = inventory
        .get("resources")?
        .as_array()?
        .iter()
        .find(|resource| resource.get("id").and_then(|value| value.as_str()) == Some(resource_id))?
        .clone();

    Some(json!({
        "summary": summary,
        "metadata": summary,
        "relationships": []
    }))
}

#[cfg(all(target_arch = "wasm32", debug_assertions))]
fn browser_preview_action(args: Option<&serde_json::Value>) -> Option<serde_json::Value> {
    let action = preview_request_value(args, "action")?;
    let request = args?.get("request")?;
    let resource_id = request
        .get("resource_id")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned);
    let name = request
        .get("payload")
        .and_then(|payload| {
            payload
                .get("bucket_name")
                .or_else(|| payload.get("table_name"))
                .or_else(|| payload.get("queue_name"))
                .or_else(|| payload.get("topic_name"))
                .or_else(|| payload.get("stream_name"))
                .or_else(|| payload.get("event_bus_name"))
                .or_else(|| payload.get("state_machine_name"))
                .or_else(|| payload.get("stack_name"))
                .or_else(|| payload.get("function_name"))
                .or_else(|| payload.get("repository_name"))
                .or_else(|| payload.get("model_id"))
                .or_else(|| payload.get("user_name"))
                .or_else(|| payload.get("role_name"))
                .or_else(|| payload.get("policy_name"))
                .or_else(|| payload.get("pool_name"))
                .or_else(|| payload.get("client_name"))
                .or_else(|| payload.get("alias_name"))
                .or_else(|| payload.get("application_name"))
                .or_else(|| payload.get("environment_name"))
                .or_else(|| payload.get("profile_name"))
                .or_else(|| payload.get("environment_id"))
                .or_else(|| payload.get("api_name"))
                .or_else(|| payload.get("path_part"))
                .or_else(|| payload.get("target_group_name"))
                .or_else(|| payload.get("target_id"))
                .or_else(|| payload.get("zone_name"))
                .or_else(|| payload.get("record_name"))
                .or_else(|| payload.get("log_group_name"))
                .or_else(|| payload.get("alarm_name"))
        })
        .and_then(|value| value.as_str())
        .map_or_else(|| resource_id.clone(), |value| Some(value.to_owned()));

    Some(json!({
        "changed": matches!(
            action,
            "create_bucket"
                | "delete_bucket"
                | "create_table"
                | "delete_table"
                | "create_queue"
                | "send_message"
                | "purge_queue"
                | "delete_queue"
                | "create_topic"
                | "publish_message"
                | "subscribe_endpoint"
                | "delete_topic"
                | "delete_subscription"
                | "create_stream"
                | "put_record"
                | "delete_stream"
                | "create_event_bus"
                | "put_event"
                | "create_rule"
                | "delete_event_bus"
                | "delete_rule"
                | "create_state_machine"
                | "start_execution"
                | "stop_execution"
                | "delete_state_machine"
                | "create_stack"
                | "delete_stack"
                | "create_function"
                | "invoke_function"
                | "delete_function"
                | "start_instance"
                | "stop_instance"
                | "terminate_instance"
                | "run_task"
                | "stop_task"
                | "update_service_desired_count"
                | "create_repository"
                | "delete_repository"
                | "delete_image"
                | "start_build"
                | "create_deployment"
                | "update_desired_capacity"
                | "invoke_model"
                | "create_user"
                | "delete_user"
                | "create_role"
                | "delete_role"
                | "create_policy"
                | "delete_policy"
                | "create_access_key"
                | "delete_access_key"
                | "create_user_pool"
                | "delete_user_pool"
                | "create_user_pool_client"
                | "delete_user_pool_client"
                | "create_key"
                | "create_alias"
                | "disable_key"
                | "schedule_key_deletion"
                | "rotate_secret"
                | "create_application"
                | "create_environment"
                | "create_configuration_profile"
                | "create_hosted_version"
                | "start_deployment"
                | "create_rest_api"
                | "create_resource"
                | "put_method"
                | "put_integration"
                | "delete_rest_api"
                | "create_api"
                | "create_route"
                | "create_integration"
                | "delete_api"
                | "create_target_group"
                | "register_target"
                | "deregister_target"
                | "delete_listener"
                | "delete_load_balancer"
                | "create_hosted_zone"
                | "upsert_record"
                | "delete_record"
                | "create_server"
                | "delete_server"
                | "create_log_group"
                | "tail_recent_events"
                | "delete_log_group"
                | "query_metric_data"
                | "create_alarm"
                | "delete_alarm"
        ),
        "message": format!("Preview completed `{action}`. Start the Tauri app to invoke Floci."),
        "resource_id": name.or(resource_id)
    }))
}

#[cfg(all(target_arch = "wasm32", debug_assertions))]
fn preview_request_value<'a>(args: Option<&'a serde_json::Value>, key: &str) -> Option<&'a str> {
    args?.get("request")?.get(key)?.as_str()
}

#[cfg(all(target_arch = "wasm32", debug_assertions))]
fn read_only_preview(
    label: &'static str,
    tabs: serde_json::Value,
    resources: serde_json::Value,
) -> (
    &'static str,
    &'static str,
    serde_json::Value,
    serde_json::Value,
    serde_json::Value,
) {
    (
        label,
        "read-only",
        tabs,
        resources,
        json!(["create", "update", "delete"]),
    )
}

#[cfg(all(target_arch = "wasm32", debug_assertions))]
fn tab<const N: usize>(
    key: &'static str,
    label: &'static str,
    kinds: [&'static str; N],
    empty_message: &'static str,
) -> serde_json::Value {
    json!({
        "key": key,
        "label": label,
        "kinds": kinds.to_vec(),
        "empty_message": empty_message
    })
}

#[cfg(all(target_arch = "wasm32", debug_assertions))]
fn resource(
    id: &'static str,
    name: &'static str,
    kind: &'static str,
    status: &'static str,
    attributes: serde_json::Value,
) -> serde_json::Value {
    json!({
        "id": id,
        "name": name,
        "kind": kind,
        "status": status,
        "created_at": null,
        "updated_at": null,
        "tags": {},
        "attributes": attributes
    })
}

#[cfg(all(target_arch = "wasm32", debug_assertions))]
fn title_case_kind(kind: &str) -> String {
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

#[cfg(all(target_arch = "wasm32", debug_assertions))]
fn is_local_browser_preview() -> bool {
    let Ok(hostname) = web_sys::window()
        .map(|window| window.location().hostname())
        .unwrap_or_else(|| Err(JsValue::NULL))
    else {
        return false;
    };

    matches!(hostname.as_str(), "localhost" | "127.0.0.1" | "::1")
}

#[cfg(all(target_arch = "wasm32", debug_assertions))]
fn browser_preview_timestamp() -> String {
    js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_else(|| "1970-01-01T00:00:00.000Z".to_owned())
}

fn js_error(value: &JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| "JavaScript interop failed.".to_owned())
}
