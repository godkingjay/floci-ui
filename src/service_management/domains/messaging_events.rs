use super::{ActionKind, ServiceActionDefinition, ServiceDomainDefinition};

pub fn definition(service_key: &str) -> Option<&'static ServiceDomainDefinition> {
    DEFINITIONS
        .iter()
        .find(|definition| definition.key == service_key)
}

pub fn is_messaging_events_service(service_key: &str) -> bool {
    definition(service_key).is_some()
}

const SQS_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_queue",
        label: "Create queue",
        kind: ActionKind::Create,
        resource_kind: Some("queue"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "send_message",
        label: "Send message",
        kind: ActionKind::Execute,
        resource_kind: Some("queue"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "purge_queue",
        label: "Purge queue",
        kind: ActionKind::Delete,
        resource_kind: Some("queue"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "refresh_queue_attributes",
        label: "Refresh attributes",
        kind: ActionKind::Refresh,
        resource_kind: Some("queue"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_queue",
        label: "Delete queue",
        kind: ActionKind::Delete,
        resource_kind: Some("queue"),
        requires_selection: true,
    },
];

const SNS_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_topic",
        label: "Create topic",
        kind: ActionKind::Create,
        resource_kind: Some("topic"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "publish_message",
        label: "Publish message",
        kind: ActionKind::Execute,
        resource_kind: Some("topic"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "subscribe_endpoint",
        label: "Subscribe endpoint",
        kind: ActionKind::Execute,
        resource_kind: Some("topic"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "refresh_topic_metadata",
        label: "Refresh metadata",
        kind: ActionKind::Refresh,
        resource_kind: Some("topic"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_topic",
        label: "Delete topic",
        kind: ActionKind::Delete,
        resource_kind: Some("topic"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_subscription",
        label: "Unsubscribe",
        kind: ActionKind::Delete,
        resource_kind: Some("subscription"),
        requires_selection: true,
    },
];

const KINESIS_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_stream",
        label: "Create stream",
        kind: ActionKind::Create,
        resource_kind: Some("stream"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "put_record",
        label: "Put record",
        kind: ActionKind::Execute,
        resource_kind: Some("stream"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "refresh_stream_summary",
        label: "Refresh summary",
        kind: ActionKind::Refresh,
        resource_kind: Some("stream"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_stream",
        label: "Delete stream",
        kind: ActionKind::Delete,
        resource_kind: Some("stream"),
        requires_selection: true,
    },
];

const EVENTBRIDGE_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_event_bus",
        label: "Create event bus",
        kind: ActionKind::Create,
        resource_kind: Some("event-bus"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "put_event",
        label: "Put event",
        kind: ActionKind::Execute,
        resource_kind: Some("event-bus"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "create_rule",
        label: "Create rule",
        kind: ActionKind::Execute,
        resource_kind: Some("event-bus"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "refresh_event_bus",
        label: "Refresh bus",
        kind: ActionKind::Refresh,
        resource_kind: Some("event-bus"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_event_bus",
        label: "Delete event bus",
        kind: ActionKind::Delete,
        resource_kind: Some("event-bus"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_rule",
        label: "Delete rule",
        kind: ActionKind::Delete,
        resource_kind: Some("rule"),
        requires_selection: true,
    },
];

const STEPFUNCTIONS_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_state_machine",
        label: "Create state machine",
        kind: ActionKind::Create,
        resource_kind: Some("state-machine"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "start_execution",
        label: "Start execution",
        kind: ActionKind::Execute,
        resource_kind: Some("state-machine"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "refresh_state_machine",
        label: "Refresh state machine",
        kind: ActionKind::Refresh,
        resource_kind: Some("state-machine"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "stop_execution",
        label: "Stop execution",
        kind: ActionKind::Delete,
        resource_kind: Some("execution"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_state_machine",
        label: "Delete state machine",
        kind: ActionKind::Delete,
        resource_kind: Some("state-machine"),
        requires_selection: true,
    },
];

const CLOUDFORMATION_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "create_stack",
        label: "Create stack",
        kind: ActionKind::Create,
        resource_kind: Some("stack"),
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "refresh_stack",
        label: "Refresh stack",
        kind: ActionKind::Refresh,
        resource_kind: Some("stack"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "view_stack_events",
        label: "View stack events",
        kind: ActionKind::Refresh,
        resource_kind: Some("stack"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "update_stack",
        label: "Update stack unsupported",
        kind: ActionKind::Unsupported,
        resource_kind: Some("stack"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_stack",
        label: "Delete stack",
        kind: ActionKind::Delete,
        resource_kind: Some("stack"),
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
        key: "send_test_email",
        label: "Send test email unsupported",
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

const SCHEDULER_ACTIONS: &[ServiceActionDefinition] = &[
    ServiceActionDefinition {
        key: "refresh_inventory",
        label: "Refresh metadata",
        kind: ActionKind::Refresh,
        resource_kind: None,
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "create_schedule",
        label: "Create schedule unsupported",
        kind: ActionKind::Unsupported,
        resource_kind: None,
        requires_selection: false,
    },
    ServiceActionDefinition {
        key: "toggle_schedule",
        label: "Enable/disable unsupported",
        kind: ActionKind::Unsupported,
        resource_kind: Some("schedule"),
        requires_selection: true,
    },
    ServiceActionDefinition {
        key: "delete_schedule",
        label: "Delete schedule unsupported",
        kind: ActionKind::Unsupported,
        resource_kind: Some("schedule"),
        requires_selection: true,
    },
];

const DEFINITIONS: &[ServiceDomainDefinition] = &[
    ServiceDomainDefinition {
        key: "sqs",
        domain_label: "Messaging, events, and workflows",
        summary: "Manage local queues, attributes, and redrive metadata with guarded create/delete flows.",
        create_field_label: Some("Queue name"),
        create_placeholder: Some("orders-events"),
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: SQS_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "sns",
        domain_label: "Messaging, events, and workflows",
        summary: "Inspect topics and subscriptions, then manage local topics with typed destructive confirmation.",
        create_field_label: Some("Topic name"),
        create_placeholder: Some("order-updates"),
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: SNS_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "ses",
        domain_label: "Messaging, events, and workflows",
        summary: "Inspect SES identities and templates while send-test actions remain disabled by default.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: READ_ONLY_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "sesv2",
        domain_label: "Messaging, events, and workflows",
        summary: "Inspect SES v2 identities, configuration sets, and contact lists without sending mail.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: READ_ONLY_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "kinesis",
        domain_label: "Messaging, events, and workflows",
        summary: "Manage local streams and inspect shard/consumer metadata without exposing record payloads.",
        create_field_label: Some("Stream name"),
        create_placeholder: Some("activity-stream"),
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: KINESIS_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "eventbridge",
        domain_label: "Messaging, events, and workflows",
        summary: "Manage event buses and inspect rule metadata while target mutation remains gated.",
        create_field_label: Some("Event bus name"),
        create_placeholder: Some("app-events"),
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: EVENTBRIDGE_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "scheduler",
        domain_label: "Messaging, events, and workflows",
        summary: "Inspect schedules and schedule groups; target-bearing mutations stay read-only for now.",
        create_field_label: None,
        create_placeholder: None,
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: SCHEDULER_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "stepfunctions",
        domain_label: "Messaging, events, and workflows",
        summary: "Manage local state machines with a safe placeholder definition and typed deletes.",
        create_field_label: Some("State machine name"),
        create_placeholder: Some("order-workflow"),
        secondary_field_label: Some("Role ARN"),
        secondary_placeholder: Some("arn:aws:iam::000000000000:role/floci-stepfunctions-local"),
        actions: STEPFUNCTIONS_ACTIONS,
    },
    ServiceDomainDefinition {
        key: "cloudformation",
        domain_label: "Messaging, events, and workflows",
        summary: "Manage local stacks with an empty-template default while change sets remain gated.",
        create_field_label: Some("Stack name"),
        create_placeholder: Some("bootstrap-stack"),
        secondary_field_label: None,
        secondary_placeholder: None,
        actions: CLOUDFORMATION_ACTIONS,
    },
];
