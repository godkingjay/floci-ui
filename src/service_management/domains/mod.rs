pub mod compute_build;
pub mod data_storage;
pub mod messaging_events;
pub mod network_observability;
pub mod security_config;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionKind {
    Create,
    Refresh,
    Execute,
    Delete,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServiceActionDefinition {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: ActionKind,
    pub resource_kind: Option<&'static str>,
    pub requires_selection: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServiceDomainDefinition {
    pub key: &'static str,
    pub domain_label: &'static str,
    pub summary: &'static str,
    pub create_field_label: Option<&'static str>,
    pub create_placeholder: Option<&'static str>,
    pub secondary_field_label: Option<&'static str>,
    pub secondary_placeholder: Option<&'static str>,
    pub actions: &'static [ServiceActionDefinition],
}

pub fn definition(service_key: &str) -> Option<&'static ServiceDomainDefinition> {
    data_storage::definition(service_key)
        .or_else(|| messaging_events::definition(service_key))
        .or_else(|| compute_build::definition(service_key))
        .or_else(|| security_config::definition(service_key))
        .or_else(|| network_observability::definition(service_key))
}

pub fn is_domain_service(service_key: &str) -> bool {
    definition(service_key).is_some()
}
