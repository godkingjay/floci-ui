use super::{ActionKind, ServiceActionDefinition, ServiceDomainDefinition};

pub fn definition(service_key: &str) -> Option<&'static ServiceDomainDefinition> {
    DEFINITIONS
        .iter()
        .find(|definition| definition.key == service_key)
}

pub fn is_data_storage_service(service_key: &str) -> bool {
    definition(service_key).is_some()
}

const S3_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_bucket",
        label: "Create bucket",
        kind: ActionKind::Create,
        resource_kind: Some("bucket"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "refresh_bucket_metadata",
        label: "Refresh metadata",
        kind: ActionKind::Refresh,
        resource_kind: Some("bucket"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "empty_bucket",
        label: "Empty bucket",
        kind: ActionKind::Unsupported,
        resource_kind: Some("bucket"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_bucket",
        label: "Delete bucket",
        kind: ActionKind::Delete,
        resource_kind: Some("bucket"),
        requires_selection: true,
    },
];

const DYNAMODB_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_table",
        label: "Create table",
        kind: ActionKind::Create,
        resource_kind: Some("table"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "refresh_table_metadata",
        label: "Refresh metadata",
        kind: ActionKind::Refresh,
        resource_kind: Some("table"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "toggle_ttl",
        label: "Toggle TTL",
        kind: ActionKind::Unsupported,
        resource_kind: Some("table"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_table",
        label: "Delete table",
        kind: ActionKind::Delete,
        resource_kind: Some("table"),
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
        key: "create",
        label: "Create unsupported",
        kind: ActionKind::Unsupported,
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

const RDS_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "refresh_inventory",
        label: "Refresh metadata",
        kind: ActionKind::Refresh,
        resource_kind: None,
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "create_snapshot",
        label: "Create snapshot unsupported",
        kind: ActionKind::Unsupported,
        resource_kind: Some("db-instance"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_snapshot",
        label: "Delete snapshot unsupported",
        kind: ActionKind::Unsupported,
        resource_kind: Some("snapshot"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_instance",
        label: "Delete instance unsupported",
        kind: ActionKind::Unsupported,
        resource_kind: Some("db-instance"),
        requires_selection: true,
    },
];

const ELASTICACHE_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "refresh_inventory",
        label: "Refresh metadata",
        kind: ActionKind::Refresh,
        resource_kind: None,
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "delete_cache_cluster",
        label: "Delete cluster unsupported",
        kind: ActionKind::Unsupported,
        resource_kind: Some("cache-cluster"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_replication_group",
        label: "Delete replication group unsupported",
        kind: ActionKind::Unsupported,
        resource_kind: Some("replication-group"),
        requires_selection: true,
    },
];

const OPENSEARCH_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "refresh_inventory",
        label: "Refresh metadata",
        kind: ActionKind::Refresh,
        resource_kind: None,
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "delete_domain",
        label: "Delete domain unsupported",
        kind: ActionKind::Unsupported,
        resource_kind: Some("domain"),
        requires_selection: true,
    },
];

const DEFINITIONS: &[ServiceDomainDefinition] = &[
    ServiceDomainDefinition {
        key: "s3",
        domain_label: "Data and storage",
        summary: "Manage local buckets with guarded destructive operations. Object body preview stays metadata-only.",
        create_field_label: Some("Bucket name"),
        create_placeholder: Some("assets-local"),
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: S3_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "dynamodb",
        domain_label: "Data and storage",
        summary: "Inspect local tables, indexes, TTL state, and backups with safe create/delete table flows.",
        create_field_label: Some("Table name"),
        create_placeholder: Some("orders"),
        secondary_field_label: Some("Partition key"),
        secondary_placeholder: Some("id"),
        actions: DYNAMODB_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "dynamodbstreams",
        domain_label: "Data and storage",
        summary: "Review stream exposure for DynamoDB tables when the local emulator exposes stream metadata.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: READ_ONLY_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "rds",
        domain_label: "Data and storage",
        summary: "Inspect local database instances, clusters, snapshots, subnet groups, and parameter groups.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: RDS_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "elasticache",
        domain_label: "Data and storage",
        summary: "Inspect cache clusters, replication groups, users, and subnet groups with unsupported destructive states.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: ELASTICACHE_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "opensearch",
        domain_label: "Data and storage",
        summary: "Inspect domains, endpoint metadata, index hints, and access policy metadata.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: OPENSEARCH_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "glue",
        domain_label: "Data and storage",
        summary: "Inspect catalog databases, tables, crawlers, jobs, and connections without rendering secrets.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: READ_ONLY_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "athena",
        domain_label: "Data and storage",
        summary: "Inspect workgroups, data catalogs, named queries, and result metadata.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: READ_ONLY_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "firehose",
        domain_label: "Data and storage",
        summary: "Inspect delivery streams and destinations while create/delete support remains gated.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: READ_ONLY_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "backup",
        domain_label: "Data and storage",
        summary: "Inspect vaults, plans, jobs, and recovery points for local backup workflows.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: READ_ONLY_ACTIONS,
    },
];
